use std::borrow::Cow;
use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Component, Path};
use std::sync::OnceLock;

use veln_ast::{
    CodecImplementationKind, Expr, ExprKind, Function, FunctionKind, Pattern, PatternKind,
    PublicAliasKind, SurfaceModule, UseDecl, Visibility, decode_surface_module, lower_surface_ast,
    lower_surface_ast_with_module_identity,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_project::{
    ManifestDependencySelectorKind, ManifestExport, ManifestField, Project, ProjectManifest,
    classify_companion_source, read_manifest,
};
use veln_source::{SourceFile, SourcePath, SourceSpan, TextRange};
use veln_syntax::{TokenKind, lex, parse};

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
        &parts.module.uses,
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
    uses: &[UseDecl],
    derived_modules: &[(String, SourceFile)],
) -> Vec<Diagnostic> {
    uses.iter()
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

#[cfg(test)]
pub(crate) fn reachable_entry_module(
    module: &SurfaceModule,
    entry: &str,
    entry_kind: FunctionKind,
) -> SurfaceModule {
    reachable_entry_module_with_cache(module, entry, entry_kind, &ReachabilityCache::default())
}

#[derive(Default)]
pub(crate) struct ReachabilityCache {
    #[cfg(test)]
    function_targets: OnceCell<ReachabilityIndex>,
    separated_function_targets: OnceCell<ReachabilityIndex>,
    direct_callees: RefCell<HashMap<ReachableFunction, Vec<ReachableFunction>>>,
}

struct ReachabilityIndex {
    function_targets: FunctionTargetIndex,
    functions_by_name: HashMap<(FunctionKind, String), Vec<FunctionRef>>,
    functions_by_qualified_name: HashMap<(FunctionKind, String, String), Vec<FunctionRef>>,
}

impl ReachabilityIndex {
    fn new(inputs: &ReachabilityInputs<'_>, function_targets: Vec<FunctionTarget>) -> Self {
        let mut functions_by_name = HashMap::<(FunctionKind, String), Vec<FunctionRef>>::new();
        let mut functions_by_qualified_name =
            HashMap::<(FunctionKind, String, String), Vec<FunctionRef>>::new();
        for function_ref in inputs.function_refs() {
            let function = inputs.function(function_ref);
            let Some(name) = &function.name else {
                continue;
            };
            functions_by_name
                .entry((function.kind, name.clone()))
                .or_default()
                .push(function_ref);
            if let Some(module_name) = &function.module_name {
                functions_by_qualified_name
                    .entry((function.kind, module_name.clone(), name.clone()))
                    .or_default()
                    .push(function_ref);
            }
        }
        Self {
            function_targets: FunctionTargetIndex::new(function_targets),
            functions_by_name,
            functions_by_qualified_name,
        }
    }

    fn function_refs(&self, key: &ReachableFunction) -> &[FunctionRef] {
        if let Some(module_name) = &key.module_name {
            self.functions_by_qualified_name
                .get(&(key.kind, module_name.clone(), key.name.clone()))
                .map(Vec::as_slice)
                .unwrap_or_default()
        } else {
            self.functions_by_name
                .get(&(key.kind, key.name.clone()))
                .map(Vec::as_slice)
                .unwrap_or_default()
        }
    }
}

#[derive(Clone, Copy)]
struct ReachabilityInputs<'a> {
    standard: Option<&'a SurfaceModule>,
    application: &'a SurfaceModule,
}

impl<'a> ReachabilityInputs<'a> {
    #[cfg(test)]
    fn combined(module: &'a SurfaceModule) -> Self {
        Self {
            standard: None,
            application: module,
        }
    }

    fn separated(standard: &'a SurfaceModule, application: &'a SurfaceModule) -> Self {
        Self {
            standard: Some(standard),
            application,
        }
    }

    fn module_header(&self) -> Option<veln_ast::ModuleHeader> {
        self.application
            .module
            .clone()
            .or_else(|| self.standard.and_then(|module| module.module.clone()))
    }

    fn cloned_declarations<T: Clone + 'a>(
        &self,
        select: impl Fn(&'a SurfaceModule) -> &'a [T],
    ) -> Vec<T> {
        self.standard
            .into_iter()
            .flat_map(|module| select(module).iter())
            .chain(select(self.application).iter())
            .cloned()
            .collect()
    }

    fn function_refs(&self) -> impl Iterator<Item = FunctionRef> + '_ {
        let standard_len = self.standard.map_or(0, |module| module.functions.len());
        (0..standard_len)
            .map(|index| FunctionRef {
                input: ReachabilityInput::Standard,
                index,
            })
            .chain(
                (0..self.application.functions.len()).map(|index| FunctionRef {
                    input: ReachabilityInput::Application,
                    index,
                }),
            )
    }

    fn functions(&self) -> impl Iterator<Item = &'a Function> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.functions.iter())
            .chain(self.application.functions.iter())
    }

    fn function(&self, function_ref: FunctionRef) -> &'a Function {
        match function_ref.input {
            ReachabilityInput::Standard => {
                &self
                    .standard
                    .expect("standard function ref should have standard input")
                    .functions[function_ref.index]
            }
            ReachabilityInput::Application => &self.application.functions[function_ref.index],
        }
    }

    fn uses(&self) -> Vec<&'a UseDecl> {
        self.standard
            .into_iter()
            .flat_map(|module| module.uses.iter())
            .chain(self.application.uses.iter())
            .collect()
    }

    fn aliases(&self) -> impl Iterator<Item = &'a veln_ast::PublicAlias> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.aliases.iter())
            .chain(self.application.aliases.iter())
    }

    fn handlers(&self) -> Vec<&'a veln_ast::HandlerDecl> {
        self.standard
            .into_iter()
            .flat_map(|module| module.handlers.iter())
            .chain(self.application.handlers.iter())
            .collect()
    }

    fn types(&self) -> impl Iterator<Item = &'a veln_ast::TypeDecl> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.types.iter())
            .chain(self.application.types.iter())
    }

    fn codecs(&self) -> impl Iterator<Item = &'a veln_ast::CodecDecl> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.codecs.iter())
            .chain(self.application.codecs.iter())
    }
}

#[derive(Clone, Copy)]
struct FunctionRef {
    input: ReachabilityInput,
    index: usize,
}

#[derive(Clone, Copy)]
enum ReachabilityInput {
    Standard,
    Application,
}

struct FunctionTargetIndex {
    all: Vec<FunctionTarget>,
    by_name: HashMap<String, Vec<usize>>,
    by_qualified_name: HashMap<(String, String), Vec<usize>>,
    by_shape: HashMap<FunctionShape, Vec<usize>>,
}

impl FunctionTargetIndex {
    fn new(all: Vec<FunctionTarget>) -> Self {
        let mut by_name = HashMap::<String, Vec<usize>>::new();
        let mut by_qualified_name = HashMap::<(String, String), Vec<usize>>::new();
        let mut by_shape = HashMap::<FunctionShape, Vec<usize>>::new();
        for (index, target) in all.iter().enumerate() {
            by_name.entry(target.name.clone()).or_default().push(index);
            if let Some(module_name) = &target.module_name {
                by_qualified_name
                    .entry((module_name.clone(), target.name.clone()))
                    .or_default()
                    .push(index);
            }
            by_shape
                .entry(target.shape.clone())
                .or_default()
                .push(index);
        }
        Self {
            all,
            by_name,
            by_qualified_name,
            by_shape,
        }
    }

    fn named(&self, name: &str) -> impl Iterator<Item = &FunctionTarget> {
        self.by_name
            .get(name)
            .into_iter()
            .flatten()
            .map(|index| &self.all[*index])
    }

    fn qualified(&self, module_name: &str, name: &str) -> impl Iterator<Item = &FunctionTarget> {
        self.by_qualified_name
            .get(&(module_name.to_string(), name.to_string()))
            .into_iter()
            .flatten()
            .map(|index| &self.all[*index])
    }

    fn shaped(&self, shape: &FunctionShape) -> impl Iterator<Item = &FunctionTarget> {
        self.by_shape
            .get(shape)
            .into_iter()
            .flatten()
            .map(|index| &self.all[*index])
    }
}

#[cfg(test)]
mod reachability_counters {
    use std::cell::Cell;

    thread_local! {
        static FUNCTION_LOOKUP_SCANS: Cell<usize> = const { Cell::new(0) };
        static TARGET_RESOLUTION_SCANS: Cell<usize> = const { Cell::new(0) };
        static MATERIALIZED_FUNCTION_BODIES: Cell<usize> = const { Cell::new(0) };
    }

    pub(super) fn reset() {
        FUNCTION_LOOKUP_SCANS.set(0);
        TARGET_RESOLUTION_SCANS.set(0);
        MATERIALIZED_FUNCTION_BODIES.set(0);
    }

    pub(super) fn record_function_lookup_scan() {
        FUNCTION_LOOKUP_SCANS.set(FUNCTION_LOOKUP_SCANS.get() + 1);
    }

    pub(super) fn record_target_resolution_scan() {
        TARGET_RESOLUTION_SCANS.set(TARGET_RESOLUTION_SCANS.get() + 1);
    }

    pub(super) fn record_materialized_function_body() {
        MATERIALIZED_FUNCTION_BODIES.set(MATERIALIZED_FUNCTION_BODIES.get() + 1);
    }

    pub(super) fn snapshot() -> (usize, usize, usize) {
        (
            FUNCTION_LOOKUP_SCANS.get(),
            TARGET_RESOLUTION_SCANS.get(),
            MATERIALIZED_FUNCTION_BODIES.get(),
        )
    }
}

#[cfg(test)]
pub(crate) fn reachable_entry_module_with_cache(
    module: &SurfaceModule,
    entry: &str,
    entry_kind: FunctionKind,
    cache: &ReachabilityCache,
) -> SurfaceModule {
    let inputs = ReachabilityInputs::combined(module);
    let reachability_index = cache
        .function_targets
        .get_or_init(|| reachable_function_targets(&inputs));
    let companion_access_targets = companion_function_access_targets(&inputs);
    let reachable = reachable_functions(
        &inputs,
        entry,
        entry_kind,
        reachability_index,
        &companion_access_targets,
        cache,
    );
    module_with_reachable_functions(&inputs, &reachable)
}

pub(crate) fn reachable_entry_module_with_standard_cache(
    standard_module: &SurfaceModule,
    application_module: &SurfaceModule,
    entry: &str,
    entry_kind: FunctionKind,
    cache: &ReachabilityCache,
) -> SurfaceModule {
    let inputs = ReachabilityInputs::separated(standard_module, application_module);
    let reachability_index = cache
        .separated_function_targets
        .get_or_init(|| reachable_function_targets(&inputs));
    let companion_access_targets = companion_function_access_targets(&inputs);
    let reachable = reachable_functions(
        &inputs,
        entry,
        entry_kind,
        reachability_index,
        &companion_access_targets,
        cache,
    );
    module_with_reachable_functions(&inputs, &reachable)
}

fn reachable_function_targets(inputs: &ReachabilityInputs<'_>) -> ReachabilityIndex {
    let mut function_targets = function_targets(inputs);
    function_targets.extend(function_alias_targets(inputs, &function_targets));
    function_targets.extend(codec_with_targets(inputs));
    ReachabilityIndex::new(inputs, function_targets)
}

fn function_targets(inputs: &ReachabilityInputs<'_>) -> Vec<FunctionTarget> {
    inputs
        .functions()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(function_target)
        .collect()
}

fn function_target(function: &Function) -> Option<FunctionTarget> {
    let name = function.name.clone()?;
    let recovery = !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase);
    Some(FunctionTarget {
        name: name.clone(),
        module_name: function.module_name.clone(),
        target_name: name,
        target_module_name: function.module_name.clone(),
        visibility: function.visibility,
        shape: function_shape(function),
        bare_importable: true,
        requires_public_import: false,
        recovery,
    })
}

fn function_shape(function: &Function) -> FunctionShape {
    let mut fixed_arity = 0usize;
    let mut variadic = None;
    for param in &function.params {
        if param.is_variadic {
            variadic = param.ty.clone();
        } else {
            fixed_arity += 1;
        }
    }
    FunctionShape {
        fixed_arity,
        variadic,
    }
}

fn codec_with_targets(inputs: &ReachabilityInputs<'_>) -> Vec<FunctionTarget> {
    inputs
        .codecs()
        .flat_map(|codec| {
            let name = codec.name.clone()?;
            Some(
                codec
                    .implementations
                    .iter()
                    .filter_map(move |implementation| {
                        let CodecImplementationKind::With {
                            function: Some(function_name),
                        } = &implementation.kind
                        else {
                            return None;
                        };
                        let target = inputs.functions().find(|function| {
                            function.kind == FunctionKind::Function
                                && function.name.as_deref() == Some(function_name.as_str())
                                && function.module_name == codec.module_name
                        })?;
                        Some(FunctionTarget {
                            name: name.clone(),
                            module_name: codec.module_name.clone(),
                            target_name: function_name.clone(),
                            target_module_name: target.module_name.clone(),
                            visibility: codec.visibility,
                            shape: function_shape(target),
                            bare_importable: false,
                            requires_public_import: true,
                            recovery: false,
                        })
                    }),
            )
        })
        .flatten()
        .collect()
}

fn reachable_functions(
    inputs: &ReachabilityInputs<'_>,
    entry: &str,
    entry_kind: FunctionKind,
    reachability_index: &ReachabilityIndex,
    companion_access_targets: &HashMap<String, String>,
    cache: &ReachabilityCache,
) -> HashSet<ReachableFunction> {
    let mut reachable = HashSet::<ReachableFunction>::new();
    let mut stack = vec![ReachableFunction {
        kind: entry_kind,
        name: entry.to_string(),
        module_name: None,
    }];

    while let Some(key) = stack.pop() {
        if !reachable.insert(key.clone()) {
            continue;
        }
        let cached_callees = cache.direct_callees.borrow().get(&key).cloned();
        let callees = cached_callees.unwrap_or_else(|| {
            let callees = reachability_index
                .function_refs(&key)
                .iter()
                .map(|function_ref| {
                    #[cfg(test)]
                    reachability_counters::record_function_lookup_scan();
                    inputs.function(*function_ref)
                })
                .flat_map(|function| {
                    direct_function_callees(
                        function,
                        inputs,
                        &reachability_index.function_targets,
                        companion_access_targets,
                    )
                })
                .collect::<Vec<_>>();
            cache
                .direct_callees
                .borrow_mut()
                .insert(key.clone(), callees.clone());
            callees
        });
        for callee in callees {
            if !reachable.contains(&callee) {
                stack.push(callee);
            }
        }
    }
    reachable
}

fn module_with_reachable_functions(
    inputs: &ReachabilityInputs<'_>,
    reachable: &HashSet<ReachableFunction>,
) -> SurfaceModule {
    let functions = materialize_reachable_functions(inputs, reachable);
    let reachable_invalid_name_spans = reachable_invalid_name_declaration_spans(inputs, &functions);
    let invalid_names_by_declaration = inputs.cloned_declarations(|module| &module.invalid_names);
    let invalid_names = inputs
        .cloned_declarations(|module| &module.invalid_names)
        .into_iter()
        .filter(|invalid| invalid_name_is_reachable(invalid, &reachable_invalid_name_spans))
        .collect();
    SurfaceModule {
        module: inputs.module_header(),
        uses: inputs.cloned_declarations(|module| &module.uses),
        aliases: inputs
            .cloned_declarations(|module| &module.aliases)
            .into_iter()
            .filter(|alias| {
                !declaration_contains_invalid_name(&alias.span, &invalid_names_by_declaration)
                    || reachable_invalid_name_spans
                        .iter()
                        .any(|span| span == &alias.span)
            })
            .collect(),
        effects: inputs.cloned_declarations(|module| &module.effects),
        handlers: inputs
            .cloned_declarations(|module| &module.handlers)
            .into_iter()
            .filter(|handler| {
                !declaration_contains_invalid_name(&handler.span, &invalid_names_by_declaration)
                    || reachable_invalid_name_spans
                        .iter()
                        .any(|span| span == &handler.span)
            })
            .collect(),
        types: inputs.cloned_declarations(|module| &module.types),
        schemas: inputs.cloned_declarations(|module| &module.schemas),
        codecs: inputs.cloned_declarations(|module| &module.codecs),
        functions,
        invalid_names,
    }
}

fn declaration_contains_invalid_name(
    declaration: &SourceSpan,
    invalid_names: &[veln_ast::InvalidName],
) -> bool {
    invalid_names
        .iter()
        .any(|invalid| span_contains(declaration, &invalid.span))
}

fn invalid_name_is_reachable(
    invalid: &veln_ast::InvalidName,
    reachable_spans: &[SourceSpan],
) -> bool {
    if let Some(span) = &invalid.enclosing_function_span {
        return reachable_spans.iter().any(|reachable| reachable == span);
    }
    reachable_spans
        .iter()
        .any(|reachable| span_contains(reachable, &invalid.span))
}

fn reachable_invalid_name_declaration_spans(
    inputs: &ReachabilityInputs<'_>,
    functions: &[Function],
) -> Vec<SourceSpan> {
    let mut selector = ReachableInvalidNameSelector::new(inputs);
    let mut spans = functions
        .iter()
        .map(|function| function.span.clone())
        .collect::<Vec<_>>();
    for function in functions {
        selector.collect_function(function, &mut spans);
    }
    dedup_spans(&mut spans);
    spans
}

struct ReachableInvalidNameSelector<'a> {
    uses: Vec<&'a UseDecl>,
    aliases: Vec<&'a veln_ast::PublicAlias>,
    handlers: Vec<&'a veln_ast::HandlerDecl>,
    types: Vec<&'a veln_ast::TypeDecl>,
    functions: Vec<&'a Function>,
}

impl<'a> ReachableInvalidNameSelector<'a> {
    fn new(inputs: &'a ReachabilityInputs<'_>) -> Self {
        Self {
            uses: inputs.uses(),
            aliases: inputs.aliases().collect(),
            handlers: inputs.handlers(),
            types: inputs.types().collect(),
            functions: inputs.functions().collect(),
        }
    }

    fn collect_function(&mut self, function: &Function, spans: &mut Vec<SourceSpan>) {
        let mut local_bindings = function
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect::<Vec<_>>();
        for param in &function.params {
            self.collect_type_annotation(
                param.ty.as_deref(),
                function.module_name.as_deref(),
                spans,
            );
        }
        self.collect_type_annotation(
            function.return_type.as_deref(),
            function.module_name.as_deref(),
            spans,
        );
        for line in &function.body {
            match &line.kind {
                veln_ast::BodyLineKind::Let {
                    pattern,
                    annotation,
                    expr,
                } => {
                    self.collect_pattern(pattern, function.module_name.as_deref(), spans);
                    self.collect_type_annotation(
                        annotation.as_deref(),
                        function.module_name.as_deref(),
                        spans,
                    );
                    self.collect_expr(
                        expr,
                        function.module_name.as_deref(),
                        &local_bindings,
                        spans,
                    );
                    collect_pattern_binding_names(pattern, &mut local_bindings);
                }
                veln_ast::BodyLineKind::Expr { expr } => {
                    self.collect_expr(
                        expr,
                        function.module_name.as_deref(),
                        &local_bindings,
                        spans,
                    );
                }
            }
        }
    }

    fn collect_type_annotation(
        &mut self,
        annotation: Option<&str>,
        current_module: Option<&str>,
        spans: &mut Vec<SourceSpan>,
    ) {
        let Some(annotation) = annotation else {
            return;
        };
        let Ok(type_names) = veln_sema::type_annotation_reference_paths(annotation) else {
            return;
        };
        for path in type_names {
            self.select_type_name(&path, current_module, spans);
        }
    }

    fn collect_expr(
        &mut self,
        expr: &Expr,
        current_module: Option<&str>,
        local_bindings: &[String],
        spans: &mut Vec<SourceSpan>,
    ) {
        match &expr.kind {
            ExprKind::NamePath(segments) => {
                if !matches!(segments.as_slice(), [name] if local_bindings.iter().rev().any(|binding| binding == name))
                {
                    self.select_value_name(segments, current_module, spans);
                }
            }
            ExprKind::Hole { .. } => {}
            ExprKind::TypeApply { callee, type_args } => {
                self.collect_expr(callee, current_module, local_bindings, spans);
                for type_arg in type_args {
                    self.collect_type_annotation(Some(type_arg), current_module, spans);
                }
            }
            ExprKind::Call { callee, args } => {
                if let Some(segments) = callee_name_path(callee) {
                    if !matches!(segments.as_slice(), [name] if local_bindings.iter().rev().any(|binding| binding == name))
                    {
                        self.select_call_name(segments, current_module, args.len(), spans);
                    }
                } else {
                    self.collect_expr(callee, current_module, local_bindings, spans);
                }
                for arg in args {
                    self.collect_expr(arg, current_module, local_bindings, spans);
                }
            }
            ExprKind::Perform { args, .. } => {
                for arg in args {
                    self.collect_expr(arg, current_module, local_bindings, spans);
                }
            }
            ExprKind::Handle {
                body,
                handler,
                args,
                ..
            } => {
                self.select_handler(handler, current_module, spans);
                self.collect_expr(body, current_module, local_bindings, spans);
                for arg in args {
                    self.collect_expr(arg, current_module, local_bindings, spans);
                }
            }
            ExprKind::SchemaDecode {
                schema: _,
                input,
                base,
            } => {
                self.collect_expr(input, current_module, local_bindings, spans);
                self.collect_expr(base, current_module, local_bindings, spans);
            }
            ExprKind::SchemaEncode { schema: _, value } => {
                self.collect_expr(value, current_module, local_bindings, spans);
            }
            ExprKind::FieldAccess { base, .. }
            | ExprKind::Try(base)
            | ExprKind::Prefix { expr: base, .. } => {
                self.collect_expr(base, current_module, local_bindings, spans);
            }
            ExprKind::Record(fields) => {
                for field in fields {
                    self.collect_expr(&field.expr, current_module, local_bindings, spans);
                }
            }
            ExprKind::Dict(entries) => {
                for entry in entries {
                    self.collect_expr(&entry.key, current_module, local_bindings, spans);
                    self.collect_expr(&entry.value, current_module, local_bindings, spans);
                }
            }
            ExprKind::List(items) => {
                for item in items {
                    self.collect_expr(item, current_module, local_bindings, spans);
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.collect_expr(scrutinee, current_module, local_bindings, spans);
                for arm in arms {
                    self.collect_pattern(&arm.pattern, current_module, spans);
                    let mut arm_bindings = local_bindings.to_vec();
                    collect_pattern_binding_names(&arm.pattern, &mut arm_bindings);
                    self.collect_expr(&arm.expr, current_module, &arm_bindings, spans);
                }
            }
            ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => {
                self.collect_expr(condition, current_module, local_bindings, spans);
                self.collect_expr(then_branch, current_module, local_bindings, spans);
                for branch in else_if_branches {
                    self.collect_expr(&branch.condition, current_module, local_bindings, spans);
                    self.collect_expr(&branch.expr, current_module, local_bindings, spans);
                }
                self.collect_expr(else_branch, current_module, local_bindings, spans);
            }
            ExprKind::Binary { left, right, .. } => {
                self.collect_expr(left, current_module, local_bindings, spans);
                self.collect_expr(right, current_module, local_bindings, spans);
            }
            ExprKind::Missing
            | ExprKind::StringLiteral(_)
            | ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::BoolLiteral(_)
            | ExprKind::Unit => {}
        }
    }

    fn collect_pattern(
        &mut self,
        pattern: &Pattern,
        current_module: Option<&str>,
        spans: &mut Vec<SourceSpan>,
    ) {
        match &pattern.kind {
            PatternKind::Binding(_) => {}
            PatternKind::Constructor { name, args } => {
                self.select_constructor_name(name, current_module, None, spans);
                for arg in args {
                    self.collect_pattern(arg, current_module, spans);
                }
            }
            PatternKind::Record(fields) => {
                for field in fields {
                    self.collect_pattern(&field.pattern, current_module, spans);
                }
            }
            PatternKind::Wildcard
            | PatternKind::StringLiteral(_)
            | PatternKind::IntLiteral(_)
            | PatternKind::FloatLiteral(_)
            | PatternKind::BoolLiteral(_)
            | PatternKind::Unit => {}
        }
    }

    fn select_value_name(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        spans: &mut Vec<SourceSpan>,
    ) {
        if self.has_valid_constructor(segments, current_module, None) {
            return;
        }
        if self.has_valid_function_alias(segments, current_module) {
            return;
        }
        if same_module_recovery_path(segments) {
            self.select_unique_constructor_recovery(segments, current_module, None, spans);
            self.select_unique_function_recovery(segments, current_module, None, spans);
        }
    }

    fn select_call_name(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: usize,
        spans: &mut Vec<SourceSpan>,
    ) {
        if self.has_valid_function(segments, current_module, Some(arg_count))
            || self.has_valid_function_alias(segments, current_module)
            || self.has_valid_constructor(segments, current_module, Some(arg_count))
        {
            return;
        }
        if same_module_recovery_path(segments) {
            self.select_unique_function_recovery(segments, current_module, Some(arg_count), spans);
            self.select_unique_constructor_recovery(
                segments,
                current_module,
                Some(arg_count),
                spans,
            );
        }
    }

    fn select_type_name(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        spans: &mut Vec<SourceSpan>,
    ) {
        if self.has_valid_type(segments, current_module)
            || self.has_valid_type_alias(segments, current_module)
        {
            return;
        }
        if same_module_recovery_path(segments) {
            self.select_unique_type_recovery(segments, current_module, spans);
        }
    }

    fn select_constructor_name(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
        spans: &mut Vec<SourceSpan>,
    ) {
        if self.has_valid_constructor(segments, current_module, arg_count) {
            return;
        }
        if same_module_recovery_path(segments) {
            self.select_unique_constructor_recovery(segments, current_module, arg_count, spans);
        }
    }

    fn select_handler(
        &mut self,
        segments: &[String],
        current_module: Option<&str>,
        spans: &mut Vec<SourceSpan>,
    ) {
        if let Some(handler) = self.visible_handler(segments, current_module) {
            if spans.iter().any(|span| span == &handler.span) {
                return;
            }
            spans.push(handler.span.clone());
            self.collect_handler(handler, spans);
        }
    }

    fn collect_handler(&mut self, handler: &veln_ast::HandlerDecl, spans: &mut Vec<SourceSpan>) {
        let current_module = handler.module_name.as_deref();
        let mut local_bindings = handler
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect::<Vec<_>>();
        for param in &handler.params {
            self.collect_type_annotation(param.ty.as_deref(), current_module, spans);
        }
        for clause in &handler.operation_clauses {
            let binding_count = local_bindings.len();
            local_bindings.extend(clause.params.iter().map(|param| param.name.clone()));
            self.collect_expr(&clause.body, current_module, &local_bindings, spans);
            local_bindings.truncate(binding_count);
        }
    }

    fn has_valid_function(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
    ) -> bool {
        self.visible_functions(segments, current_module)
            .into_iter()
            .any(|function| {
                function
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_lowercase))
                    && arg_count
                        .is_none_or(|count| function_shape(function).accepts_arg_count(count))
            })
    }

    fn has_valid_function_alias(&self, segments: &[String], current_module: Option<&str>) -> bool {
        self.visible_aliases(segments, current_module, PublicAliasKind::Function)
            .into_iter()
            .any(|alias| {
                alias
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_lowercase))
            })
    }

    fn has_valid_type_alias(&self, segments: &[String], current_module: Option<&str>) -> bool {
        self.visible_aliases(segments, current_module, PublicAliasKind::Type)
            .into_iter()
            .any(|alias| {
                alias
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_uppercase))
            })
    }

    fn has_valid_type(&self, segments: &[String], current_module: Option<&str>) -> bool {
        self.visible_types(segments, current_module)
            .into_iter()
            .any(|type_decl| {
                type_decl
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_uppercase))
            })
    }

    fn has_valid_constructor(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
    ) -> bool {
        self.visible_constructor_variants(segments, current_module)
            .into_iter()
            .any(|(_, variant)| {
                variant
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_uppercase))
                    && arg_count.is_none_or(|count| variant.fields.len() == count)
            })
    }

    fn select_unique_function_recovery(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
        spans: &mut Vec<SourceSpan>,
    ) {
        let candidates = self
            .visible_functions(segments, current_module)
            .into_iter()
            .filter(|function| {
                function.name.as_ref().is_some_and(|name| {
                    !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                }) && arg_count
                    .is_none_or(|count| function_shape(function).accepts_arg_count(count))
            })
            .map(|function| function.span.clone())
            .chain(
                self.visible_aliases(segments, current_module, PublicAliasKind::Function)
                    .into_iter()
                    .filter(|alias| {
                        alias.name.as_ref().is_some_and(|name| {
                            !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                        })
                    })
                    .map(|alias| alias.span.clone()),
            )
            .collect::<Vec<_>>();
        push_unique_span(candidates, spans);
    }

    fn select_unique_type_recovery(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        spans: &mut Vec<SourceSpan>,
    ) {
        let candidates = self
            .visible_types(segments, current_module)
            .into_iter()
            .filter(|type_decl| {
                type_decl.name.as_ref().is_some_and(|name| {
                    !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
                })
            })
            .map(|type_decl| type_decl.span.clone())
            .chain(
                self.visible_aliases(segments, current_module, PublicAliasKind::Type)
                    .into_iter()
                    .filter(|alias| {
                        alias.name.as_ref().is_some_and(|name| {
                            !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
                        })
                    })
                    .map(|alias| alias.span.clone()),
            )
            .collect::<Vec<_>>();
        push_unique_span(candidates, spans);
    }

    fn select_unique_constructor_recovery(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
        spans: &mut Vec<SourceSpan>,
    ) {
        let candidates = self
            .visible_constructor_variants(segments, current_module)
            .into_iter()
            .filter(|(_, variant)| {
                variant.name.as_ref().is_some_and(|name| {
                    !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
                }) && arg_count.is_none_or(|count| variant.fields.len() == count)
            })
            .map(|(type_decl, _)| type_decl.span.clone())
            .collect::<Vec<_>>();
        push_unique_span(candidates, spans);
    }

    fn visible_functions(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Vec<&'a Function> {
        let target = visible_path_target(&self.uses, segments, current_module);
        let leaf = path_leaf(segments).map(str::to_string);
        self.functions
            .iter()
            .copied()
            .filter(move |function| {
                function.kind == FunctionKind::Function
                    && function.name.as_deref() == leaf.as_deref()
                    && declaration_visible(
                        function.module_name.as_deref(),
                        function.visibility,
                        target.as_deref(),
                        current_module,
                    )
            })
            .collect()
    }

    fn visible_aliases(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        kind: PublicAliasKind,
    ) -> Vec<&'a veln_ast::PublicAlias> {
        let target = visible_path_target(&self.uses, segments, current_module);
        let leaf = path_leaf(segments).map(str::to_string);
        self.aliases
            .iter()
            .copied()
            .filter(move |alias| {
                alias.kind == kind
                    && alias.name.as_deref() == leaf.as_deref()
                    && declaration_visible(
                        alias.module_name.as_deref(),
                        Visibility::Public,
                        target.as_deref(),
                        current_module,
                    )
            })
            .collect()
    }

    fn visible_types(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Vec<&'a veln_ast::TypeDecl> {
        let target = visible_path_target(&self.uses, segments, current_module);
        let leaf = path_leaf(segments).map(str::to_string);
        self.types
            .iter()
            .copied()
            .filter(move |type_decl| {
                type_decl.name.as_deref() == leaf.as_deref()
                    && declaration_visible(
                        type_decl.module_name.as_deref(),
                        type_decl.visibility,
                        target.as_deref(),
                        current_module,
                    )
            })
            .collect()
    }

    fn visible_constructor_variants(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Vec<(&'a veln_ast::TypeDecl, &'a veln_ast::TypeVariantDecl)> {
        let target = visible_path_target(&self.uses, segments, current_module);
        let leaf = path_leaf(segments).map(str::to_string);
        self.types
            .iter()
            .copied()
            .flat_map(move |type_decl| {
                type_decl.variants.iter().filter_map({
                    let target = target.clone();
                    let leaf = leaf.clone();
                    move |variant| {
                        (variant.name.as_deref() == leaf.as_deref()
                            && declaration_visible(
                                type_decl.module_name.as_deref(),
                                type_decl.visibility,
                                target.as_deref(),
                                current_module,
                            ))
                        .then_some((type_decl, variant))
                    }
                })
            })
            .collect()
    }

    fn visible_handler(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<&'a veln_ast::HandlerDecl> {
        let target = visible_path_target(&self.uses, segments, current_module);
        self.handlers.iter().copied().find(|handler| {
            handler.name.as_deref() == path_leaf(segments)
                && declaration_visible(
                    handler.module_name.as_deref(),
                    handler.visibility,
                    target.as_deref(),
                    current_module,
                )
        })
    }
}

impl FunctionShape {
    fn accepts_arg_count(&self, arg_count: usize) -> bool {
        self.variadic.is_some() && arg_count >= self.fixed_arity
            || self.variadic.is_none() && arg_count == self.fixed_arity
    }
}

fn visible_path_target(
    uses: &[&UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<String> {
    match segments {
        [_] => current_module.map(str::to_string),
        [_, .., _] => imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
            .map(|use_decl| use_decl.name.clone()),
        _ => None,
    }
}

fn path_leaf(segments: &[String]) -> Option<&str> {
    segments.last().map(String::as_str)
}

fn same_module_recovery_path(segments: &[String]) -> bool {
    matches!(segments, [_])
}

fn declaration_visible(
    declaration_module: Option<&str>,
    visibility: Visibility,
    target_module: Option<&str>,
    current_module: Option<&str>,
) -> bool {
    match target_module {
        Some(target_module) if Some(target_module) != current_module => {
            declaration_module == Some(target_module) && visibility == Visibility::Public
        }
        Some(target_module) => declaration_module == Some(target_module),
        None => current_module.is_none() && declaration_module.is_none(),
    }
}

fn push_unique_span(mut candidates: Vec<SourceSpan>, spans: &mut Vec<SourceSpan>) {
    dedup_spans(&mut candidates);
    if let [span] = candidates.as_slice() {
        spans.push(span.clone());
    }
}

fn dedup_spans(spans: &mut Vec<SourceSpan>) {
    let mut seen = Vec::<SourceSpan>::new();
    spans.retain(|span| {
        if seen.iter().any(|known| known == span) {
            false
        } else {
            seen.push(span.clone());
            true
        }
    });
}

fn collect_pattern_binding_names(pattern: &Pattern, bindings: &mut Vec<String>) {
    match &pattern.kind {
        PatternKind::Binding(name) => bindings.push(name.clone()),
        PatternKind::Constructor { args, .. } => {
            for arg in args {
                collect_pattern_binding_names(arg, bindings);
            }
        }
        PatternKind::Record(fields) => {
            for field in fields {
                collect_pattern_binding_names(&field.pattern, bindings);
            }
        }
        PatternKind::Wildcard
        | PatternKind::StringLiteral(_)
        | PatternKind::IntLiteral(_)
        | PatternKind::FloatLiteral(_)
        | PatternKind::BoolLiteral(_)
        | PatternKind::Unit => {}
    }
}

fn span_contains(container: &SourceSpan, span: &SourceSpan) -> bool {
    container.file == span.file
        && container.start.offset <= span.start.offset
        && span.end.offset <= container.end.offset
}

fn materialize_reachable_functions(
    inputs: &ReachabilityInputs<'_>,
    reachable: &HashSet<ReachableFunction>,
) -> Vec<Function> {
    inputs
        .functions()
        .filter(|function| {
            function.name.as_ref().is_some_and(|name| {
                reachable.contains(&ReachableFunction {
                    kind: function.kind,
                    name: name.clone(),
                    module_name: None,
                }) || reachable.contains(&ReachableFunction {
                    kind: function.kind,
                    name: name.clone(),
                    module_name: function.module_name.clone(),
                })
            })
        })
        .inspect(|_function| {
            #[cfg(test)]
            if !_function.body.is_empty() {
                reachability_counters::record_materialized_function_body();
            }
        })
        .cloned()
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ReachableFunction {
    kind: FunctionKind,
    name: String,
    module_name: Option<String>,
}

#[derive(Clone, Debug)]
struct FunctionTarget {
    name: String,
    module_name: Option<String>,
    target_name: String,
    target_module_name: Option<String>,
    visibility: Visibility,
    shape: FunctionShape,
    bare_importable: bool,
    requires_public_import: bool,
    recovery: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct FunctionShape {
    fixed_arity: usize,
    variadic: Option<String>,
}

fn function_alias_targets(
    inputs: &ReachabilityInputs<'_>,
    function_targets: &[FunctionTarget],
) -> Vec<FunctionTarget> {
    let uses = inputs.uses();
    inputs
        .aliases()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
        .filter_map(|alias| {
            let name = alias.name.clone()?;
            let recovery = !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase);
            let target = target_for_alias_path(
                &alias.target,
                &uses,
                function_targets,
                alias.module_name.as_deref(),
            )?;
            if companion_alias_targets_imported_private_function(alias, target) {
                return None;
            }
            if target.recovery {
                return None;
            }
            Some(FunctionTarget {
                name,
                module_name: alias.module_name.clone(),
                target_name: target.target_name.clone(),
                target_module_name: target.target_module_name.clone(),
                visibility: Visibility::Public,
                shape: target.shape.clone(),
                bare_importable: true,
                requires_public_import: false,
                recovery,
            })
        })
        .collect()
}

fn companion_alias_targets_imported_private_function(
    alias: &veln_ast::PublicAlias,
    target: &FunctionTarget,
) -> bool {
    target.visibility != Visibility::Public
        && alias.module_name != target.target_module_name
        && classify_companion_source(alias.span.file.as_str()).is_some()
}

fn target_for_alias_path<'a>(
    segments: &[String],
    uses: &[&UseDecl],
    function_targets: &'a [FunctionTarget],
    current_module: Option<&str>,
) -> Option<&'a FunctionTarget> {
    match segments {
        [name] => function_targets.iter().find(|target| target.name == *name),
        [_, .., name] => {
            let use_decl =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
            let module_name = use_decl.name.as_str();
            function_targets.iter().find(|target| {
                target.name == *name
                    && target.module_name.as_deref() == Some(module_name)
                    && imported_target_is_visible(target, use_decl)
            })
        }
        _ => None,
    }
}

#[derive(Clone, Debug)]
struct LocalBinding {
    name: String,
    function_shape: Option<FunctionShape>,
}

struct FunctionCalleeContext<'a> {
    current_module: Option<&'a str>,
    uses: &'a [&'a UseDecl],
    function_targets: &'a FunctionTargetIndex,
    companion_access_targets: &'a HashMap<String, String>,
    handlers: &'a [&'a veln_ast::HandlerDecl],
    types: &'a [&'a veln_ast::TypeDecl],
}

fn direct_function_callees(
    function: &Function,
    inputs: &ReachabilityInputs<'_>,
    function_targets: &FunctionTargetIndex,
    companion_access_targets: &HashMap<String, String>,
) -> Vec<ReachableFunction> {
    let mut callees = Vec::new();
    let uses = inputs.uses();
    let handlers = inputs.handlers();
    let types = inputs.types().collect::<Vec<_>>();
    let context = FunctionCalleeContext {
        current_module: function.module_name.as_deref(),
        uses: &uses,
        function_targets,
        companion_access_targets,
        handlers: &handlers,
        types: &types,
    };
    let mut local_bindings = function
        .params
        .iter()
        .map(|param| LocalBinding {
            name: param.name.clone(),
            function_shape: param.ty.as_deref().and_then(function_type_shape),
        })
        .collect::<Vec<_>>();
    for contract in &function.contracts {
        collect_contract_callees(
            &contract.text,
            context.current_module,
            context.uses,
            function_targets,
            companion_access_targets,
            &mut callees,
        );
    }
    for line in &function.body {
        match &line.kind {
            veln_ast::BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                collect_function_callees(expr, &context, &local_bindings, &mut callees);
                collect_pattern_bindings(
                    pattern,
                    annotation.as_deref().and_then(function_type_shape),
                    &mut local_bindings,
                );
            }
            veln_ast::BodyLineKind::Expr { expr } => {
                collect_function_callees(expr, &context, &local_bindings, &mut callees);
            }
        }
    }
    callees
}

fn collect_contract_callees(
    predicate: &str,
    current_module: Option<&str>,
    uses: &[&UseDecl],
    function_targets: &FunctionTargetIndex,
    companion_access_targets: &HashMap<String, String>,
    callees: &mut Vec<ReachableFunction>,
) {
    let source = SourceFile::new("<contract>", predicate);
    let tokens = lex(&source)
        .tokens
        .into_iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
        .collect::<Vec<_>>();
    let mut index = 0usize;
    while index < tokens.len() {
        let name = &tokens[index];
        if name.kind != TokenKind::Ident {
            index += 1;
            continue;
        }
        let mut segments = vec![name.text.clone()];
        let mut next_index = index + 1;
        while next_index + 1 < tokens.len()
            && tokens[next_index].kind == TokenKind::DoubleColon
            && tokens[next_index + 1].kind == TokenKind::Ident
        {
            segments.push(tokens[next_index + 1].text.clone());
            next_index += 2;
        }
        let Some(next) = tokens.get(next_index) else {
            break;
        };
        if next.kind != TokenKind::LParen {
            index += 1;
            continue;
        }
        for callee in resolve_function_reference(
            &segments,
            current_module,
            uses,
            function_targets,
            companion_access_targets,
        ) {
            push_reachable(callees, callee);
        }
        index = next_index + 1;
    }
    collect_contract_function_value_references(
        &tokens,
        current_module,
        uses,
        function_targets,
        companion_access_targets,
        callees,
    );
}

fn collect_contract_function_value_references(
    tokens: &[veln_syntax::Token],
    current_module: Option<&str>,
    uses: &[&UseDecl],
    function_targets: &FunctionTargetIndex,
    _companion_access_targets: &HashMap<String, String>,
    callees: &mut Vec<ReachableFunction>,
) {
    let mut index = 0usize;
    while index < tokens.len() {
        if tokens[index].kind != TokenKind::Ident {
            index += 1;
            continue;
        }
        if index > 0
            && matches!(
                tokens[index - 1].kind,
                TokenKind::Dot | TokenKind::DoubleColon
            )
        {
            index += 1;
            continue;
        }
        if tokens
            .get(index + 1)
            .is_some_and(|token| matches!(token.kind, TokenKind::Dot | TokenKind::LParen))
        {
            index += 1;
            continue;
        }
        let segments = if tokens
            .get(index + 1)
            .is_some_and(|token| token.kind == TokenKind::DoubleColon)
            && tokens
                .get(index + 2)
                .is_some_and(|token| token.kind == TokenKind::Ident)
        {
            let mut segments = vec![tokens[index].text.clone()];
            index += 1;
            while tokens
                .get(index)
                .is_some_and(|token| token.kind == TokenKind::DoubleColon)
                && tokens
                    .get(index + 1)
                    .is_some_and(|token| token.kind == TokenKind::Ident)
            {
                segments.push(tokens[index + 1].text.clone());
                index += 2;
            }
            segments
        } else {
            let segments = vec![tokens[index].text.clone()];
            index += 1;
            segments
        };
        let public_or_same_module_access = HashMap::new();
        for callee in resolve_function_reference(
            &segments,
            current_module,
            uses,
            function_targets,
            &public_or_same_module_access,
        ) {
            push_reachable(callees, callee);
        }
    }
}

fn collect_function_callees(
    expr: &Expr,
    context: &FunctionCalleeContext<'_>,
    local_bindings: &[LocalBinding],
    callees: &mut Vec<ReachableFunction>,
) {
    let current_module = context.current_module;
    let uses = context.uses;
    let function_targets = context.function_targets;
    let companion_access_targets = context.companion_access_targets;
    let handlers = context.handlers;

    match &expr.kind {
        ExprKind::NamePath(segments) => {
            collect_function_name_reference(segments, context, local_bindings, None, callees);
        }
        ExprKind::TypeApply { callee, .. } => {
            collect_function_callees(callee, context, local_bindings, callees);
        }
        ExprKind::Call { callee, args } => {
            if let Some(segments) = callee_name_path(callee) {
                collect_function_name_reference(
                    segments,
                    context,
                    local_bindings,
                    Some(args.len()),
                    callees,
                );
            } else {
                collect_function_callees(callee, context, local_bindings, callees);
            }
            for arg in args {
                collect_function_callees(arg, context, local_bindings, callees);
            }
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_function_callees(arg, context, local_bindings, callees);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_handler_operation_clause_callees(
                expr,
                current_module,
                uses,
                function_targets,
                companion_access_targets,
                handlers,
                callees,
            );
            collect_function_callees(body, context, local_bindings, callees);
            for arg in args {
                collect_function_callees(arg, context, local_bindings, callees);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_function_callees(input, context, local_bindings, callees);
            collect_function_callees(base, context, local_bindings, callees);
        }
        ExprKind::SchemaEncode { value, .. } => {
            collect_function_callees(value, context, local_bindings, callees);
        }
        ExprKind::FieldAccess { base, .. } => {
            collect_function_callees(base, context, local_bindings, callees);
        }
        ExprKind::Try(inner) => collect_function_callees(inner, context, local_bindings, callees),
        ExprKind::Record(fields) => {
            for field in fields {
                collect_function_callees(&field.expr, context, local_bindings, callees);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_function_callees(&entry.key, context, local_bindings, callees);
                collect_function_callees(&entry.value, context, local_bindings, callees);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                collect_function_callees(item, context, local_bindings, callees);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_function_callees(scrutinee, context, local_bindings, callees);
            for arm in arms {
                let mut arm_bindings = local_bindings.to_vec();
                collect_pattern_bindings(&arm.pattern, None, &mut arm_bindings);
                collect_function_callees(&arm.expr, context, &arm_bindings, callees);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_function_callees(condition, context, local_bindings, callees);
            collect_function_callees(then_branch, context, local_bindings, callees);
            for branch in else_if_branches {
                collect_function_callees(&branch.condition, context, local_bindings, callees);
                collect_function_callees(&branch.expr, context, local_bindings, callees);
            }
            collect_function_callees(else_branch, context, local_bindings, callees);
        }
        ExprKind::Prefix { expr, .. } => {
            collect_function_callees(expr, context, local_bindings, callees);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_function_callees(left, context, local_bindings, callees);
            collect_function_callees(right, context, local_bindings, callees);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
    }
}

fn collect_pattern_bindings(
    pattern: &Pattern,
    function_shape: Option<FunctionShape>,
    bindings: &mut Vec<LocalBinding>,
) {
    match &pattern.kind {
        PatternKind::Binding(name) => bindings.push(LocalBinding {
            name: name.clone(),
            function_shape,
        }),
        PatternKind::Record(fields) => {
            for field in fields {
                collect_pattern_bindings(&field.pattern, None, bindings);
            }
        }
        PatternKind::Constructor { args, .. } => {
            for arg in args {
                collect_pattern_bindings(arg, None, bindings);
            }
        }
        PatternKind::Wildcard
        | PatternKind::StringLiteral(_)
        | PatternKind::IntLiteral(_)
        | PatternKind::FloatLiteral(_)
        | PatternKind::BoolLiteral(_)
        | PatternKind::Unit => {}
    }
}

fn callee_name_path(callee: &Expr) -> Option<&Vec<String>> {
    match &callee.kind {
        ExprKind::NamePath(segments) => Some(segments),
        ExprKind::TypeApply { callee, .. } => callee_name_path(callee),
        _ => None,
    }
}

fn collect_opaque_function_value_callees(
    shape: &FunctionShape,
    arg_count: Option<usize>,
    current_module: Option<&str>,
    uses: &[&UseDecl],
    function_targets: &FunctionTargetIndex,
    _companion_access_targets: &HashMap<String, String>,
    callees: &mut Vec<ReachableFunction>,
) {
    if current_module.is_some_and(|module| module.starts_with("std::")) {
        return;
    }
    if shape.variadic.is_some() && arg_count.is_some_and(|arg_count| arg_count < shape.fixed_arity)
    {
        return;
    }
    let public_or_same_module_access = HashMap::new();
    for target in function_targets.shaped(shape).filter(|target| {
        target_visible_from_current_module(
            target,
            current_module,
            uses,
            &public_or_same_module_access,
        )
    }) {
        push_reachable(
            callees,
            ReachableFunction {
                kind: FunctionKind::Function,
                name: target.name.clone(),
                module_name: target.module_name.clone(),
            },
        );
    }
}

fn target_visible_from_current_module(
    target: &FunctionTarget,
    current_module: Option<&str>,
    uses: &[&UseDecl],
    companion_access_targets: &HashMap<String, String>,
) -> bool {
    let target_module = target.module_name.as_deref();
    if current_module.is_none() || target_module == current_module {
        return true;
    }
    target_module.is_some_and(|module_name| {
        uses.iter().any(|use_decl| {
            use_decl.module_name.as_deref() == current_module
                && use_decl.origin == veln_ast::UseOrigin::Source
                && use_decl.name == module_name
                && imported_target_visible_from_module(
                    target,
                    use_decl,
                    current_module,
                    companion_access_targets,
                )
        })
    })
}

fn function_type_shape(annotation: &str) -> Option<FunctionShape> {
    let params = annotation.trim().strip_prefix("fn")?.trim_start();
    let params = params.strip_prefix('(')?;
    let mut depth = 0usize;
    let mut split_at = None;
    for (index, ch) in params.char_indices() {
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' if depth == 0 => {
                split_at = Some(index);
                break;
            }
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    let params = &params[..split_at?].trim();
    if params.is_empty() {
        return Some(FunctionShape {
            fixed_arity: 0,
            variadic: None,
        });
    }
    let mut parts = split_top_level_commas(params);
    let variadic = parts.last().and_then(|last| {
        last.strip_prefix("...")
            .map(str::trim)
            .filter(|element| !element.is_empty())
            .map(str::to_string)
    });
    if variadic.is_some() {
        parts.pop();
    }
    Some(FunctionShape {
        fixed_arity: parts.len(),
        variadic,
    })
}

fn split_top_level_commas(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());
    parts
}

fn path_has_valid_constructor(
    segments: &[String],
    arg_count: Option<usize>,
    current_module: Option<&str>,
    uses: &[&UseDecl],
    types: &[&veln_ast::TypeDecl],
) -> bool {
    let target = visible_path_target(uses, segments, current_module);
    let leaf = path_leaf(segments);
    types.iter().copied().any(|type_decl| {
        declaration_visible(
            type_decl.module_name.as_deref(),
            type_decl.visibility,
            target.as_deref(),
            current_module,
        ) && type_decl.variants.iter().any(|variant| {
            variant.name.as_deref() == leaf
                && variant
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_uppercase))
                && arg_count.is_none_or(|count| variant.fields.len() == count)
        })
    })
}

fn collect_function_name_reference(
    segments: &[String],
    context: &FunctionCalleeContext<'_>,
    local_bindings: &[LocalBinding],
    arg_count: Option<usize>,
    callees: &mut Vec<ReachableFunction>,
) {
    let current_module = context.current_module;
    let uses = context.uses;
    let function_targets = context.function_targets;
    let companion_access_targets = context.companion_access_targets;
    let types = context.types;

    if let [name] = segments
        && let Some(binding) = local_bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)
    {
        if let Some(shape) = &binding.function_shape {
            collect_opaque_function_value_callees(
                shape,
                arg_count,
                current_module,
                uses,
                function_targets,
                companion_access_targets,
                callees,
            );
        }
        return;
    }
    if path_has_valid_constructor(segments, arg_count, current_module, uses, types) {
        return;
    }
    let public_or_same_module_access;
    let access_targets = if arg_count.is_some() {
        companion_access_targets
    } else {
        public_or_same_module_access = HashMap::new();
        &public_or_same_module_access
    };
    for callee in resolve_function_reference(
        segments,
        current_module,
        uses,
        function_targets,
        access_targets,
    ) {
        push_reachable(callees, callee);
    }
}

fn collect_handler_operation_clause_callees(
    expr: &Expr,
    current_module: Option<&str>,
    uses: &[&UseDecl],
    function_targets: &FunctionTargetIndex,
    companion_access_targets: &HashMap<String, String>,
    handlers: &[&veln_ast::HandlerDecl],
    callees: &mut Vec<ReachableFunction>,
) {
    let ExprKind::Handle { handler, .. } = &expr.kind else {
        return;
    };
    let matching_handlers = handlers.iter().filter(|candidate| {
        let Some(name) = &candidate.name else {
            return false;
        };
        match handler.as_slice() {
            [segment] => name == segment && candidate.module_name.as_deref() == current_module,
            [_, .., segment] => {
                let Some(use_decl) =
                    imported_use_for_path(uses, &handler[..handler.len() - 1], current_module)
                else {
                    return false;
                };
                name == segment && candidate.module_name.as_deref() == Some(use_decl.name.as_str())
            }
            _ => false,
        }
    });
    for handler in matching_handlers {
        let context = FunctionCalleeContext {
            current_module,
            uses,
            function_targets,
            companion_access_targets,
            handlers,
            types: &[],
        };
        let mut local_bindings = handler
            .params
            .iter()
            .map(|param| LocalBinding {
                name: param.name.clone(),
                function_shape: param.ty.as_deref().and_then(function_type_shape),
            })
            .collect::<Vec<_>>();
        for clause in &handler.operation_clauses {
            let binding_count = local_bindings.len();
            local_bindings.extend(clause.params.iter().map(|param| LocalBinding {
                name: param.name.clone(),
                function_shape: None,
            }));
            collect_function_callees(&clause.body, &context, &local_bindings, callees);
            local_bindings.truncate(binding_count);
        }
    }
}

fn resolve_function_reference(
    segments: &[String],
    current_module: Option<&str>,
    uses: &[&UseDecl],
    function_targets: &FunctionTargetIndex,
    companion_access_targets: &HashMap<String, String>,
) -> Vec<ReachableFunction> {
    match segments {
        [name] => function_targets
            .named(name)
            .filter(|target| {
                #[cfg(test)]
                reachability_counters::record_target_resolution_scan();
                target.name == *name && bare_target_visible(target, current_module, uses)
            })
            .map(|target| ReachableFunction {
                kind: FunctionKind::Function,
                name: target.target_name.clone(),
                module_name: target.target_module_name.clone(),
            })
            .collect(),
        [_, .., name] => {
            let Some(use_decl) =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
            else {
                return Vec::new();
            };
            let module_name = use_decl.name.as_str();
            function_targets
                .qualified(module_name, name)
                .filter(|target| {
                    #[cfg(test)]
                    reachability_counters::record_target_resolution_scan();
                    imported_target_visible_from_module(
                        target,
                        use_decl,
                        current_module,
                        companion_access_targets,
                    )
                })
                .map(|target| ReachableFunction {
                    kind: FunctionKind::Function,
                    name: target.target_name.clone(),
                    module_name: target.target_module_name.clone(),
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

fn imported_use_for_path<'a>(
    uses: &'a [&'a UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let module_path = segments.join("::");
    uses.iter().copied().find(|use_decl| {
        use_decl.module_name.as_deref() == current_module
            && (use_decl.name == module_path || use_decl.alias == module_path)
    })
}

fn imported_target_is_visible(target: &FunctionTarget, use_decl: &UseDecl) -> bool {
    if target.requires_public_import {
        return target.visibility == Visibility::Public;
    }
    use_decl.package.is_none() || target.visibility == Visibility::Public
}

fn imported_target_visible_from_module(
    target: &FunctionTarget,
    use_decl: &UseDecl,
    current_module: Option<&str>,
    companion_access_targets: &HashMap<String, String>,
) -> bool {
    if target.recovery {
        return false;
    }
    if target.visibility == Visibility::Public {
        return true;
    }
    if target.requires_public_import || use_decl.package.is_some() {
        return false;
    }
    if current_module.is_some_and(|module| module.starts_with("std::"))
        && target
            .module_name
            .as_deref()
            .is_some_and(|module| module.starts_with("std::"))
    {
        return true;
    }
    current_module.is_some_and(|current_module| {
        target.module_name.as_ref().is_some_and(|target_module| {
            companion_access_targets
                .get(current_module)
                .is_some_and(|allowed_target| allowed_target == target_module)
        })
    })
}

fn companion_function_access_targets(inputs: &ReachabilityInputs<'_>) -> HashMap<String, String> {
    inputs
        .functions()
        .filter_map(|function| {
            let companion = classify_companion_source(function.span.file.as_str())?;
            let companion_module = function.module_name.clone()?;
            let target_module = companion
                .target_path
                .strip_suffix(".veln")?
                .replace('/', "::");
            Some((companion_module, target_module))
        })
        .collect()
}

fn bare_target_visible(
    target: &FunctionTarget,
    current_module: Option<&str>,
    uses: &[&UseDecl],
) -> bool {
    let Some(current_module) = current_module else {
        return true;
    };
    if target.module_name.as_deref() == Some(current_module) {
        return true;
    }
    if target.recovery {
        return false;
    }
    target.bare_importable
        && target.module_name.as_deref().is_some_and(|module_name| {
            uses.iter().any(|use_decl| {
                use_decl.module_name.as_deref() == Some(current_module)
                    && use_decl.name == module_name
                    && imported_target_is_visible(target, use_decl)
            })
        })
}

fn push_reachable(callees: &mut Vec<ReachableFunction>, callee: ReachableFunction) {
    if !callees.iter().any(|known| known == &callee) {
        callees.push(callee);
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::{env, fs};

    use veln_ast::{
        CodecDecl, CodecDirection, CodecImplementationClause, CodecImplementationKind,
        FunctionKind, SurfaceModule, UseOrigin, Visibility, lower_surface_ast,
    };
    use veln_project::{
        ManifestExport, ManifestField, ManifestLib, ManifestTool, ManifestUnsupportedSection,
        Project, ProjectManifest, parse_manifest_text,
    };
    use veln_source::{LineCol, SourceFile, SourcePath, SourceSpan};
    use veln_syntax::parse;

    use super::{
        Diagnostic, EmbeddedStandardModuleEntry, EmbeddedStandardPackage, ReachabilityCache,
        SurfaceParts, embedded_standard_counters, load_embedded_standard_package_from,
        load_project_sources, load_surface_module, reachability_counters, reachable_entry_module,
        reachable_entry_module_with_standard_cache, validate_manifest_exports,
    };

    fn lower(text: &str) -> SurfaceModule {
        let source = SourceFile::new("main_test.veln", text);
        let parsed = parse(&source);
        assert!(
            parsed.diagnostics.is_empty(),
            "unexpected parse diagnostics: {:?}",
            parsed.diagnostics
        );
        lower_surface_ast(&parsed.tree)
    }

    fn reachable_function_names(module: &SurfaceModule) -> Vec<(&str, &str)> {
        let mut functions = module
            .functions
            .iter()
            .filter_map(|function| {
                Some((function.module_name.as_deref()?, function.name.as_deref()?))
            })
            .collect::<Vec<_>>();
        functions.sort_unstable();
        functions
    }

    struct TempProject {
        root: PathBuf,
    }

    impl TempProject {
        fn new(name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after Unix epoch")
                .as_nanos();
            let root = env::temp_dir().join(format!(
                "veln-analysis-surface-{name}-{}-{unique}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).expect("temporary project root should be created");
            Self { root }
        }

        fn root(&self) -> &Path {
            &self.root
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.root.join(relative)
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.path(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("temporary project parent should be created");
            }
            fs::write(path, contents).expect("temporary project file should be written");
        }
    }

    impl Drop for TempProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn external_path_dependency_loads_direct_manifest_package_root() {
        let temp = TempProject::new("external-path-dependency-root");
        temp.write(
            "veln.toml",
            "[dependencies.\"github.com/oakcask/foo\"]\npath = \"vendor/foo\"\n",
        );
        temp.write(
            "main.veln",
            "use foo from \"github.com/oakcask/foo\"\n\npub fn main() -> Int\n  add_one(1)\nend\n",
        );
        temp.write(
            "vendor/foo/veln.toml",
            "[package]\nname = \"github.com/oakcask/foo\"\n\n[lib]\nexports = [\"foo.veln\"]\n",
        );
        temp.write(
            "vendor/foo/foo.veln",
            "pub fn add_one(value: Int) -> Int\n  value + 1\nend\n",
        );

        let project =
            Project::discover(temp.root().to_path_buf(), &[PathBuf::from("main.veln")]).unwrap();
        let (_, diagnostics) = load_surface_module(&project);

        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }

    #[test]
    fn external_git_dependency_loads_materialized_subdir_package_root() {
        let temp = TempProject::new("external-git-dependency-subdir-root");
        temp.write(
            "veln.toml",
            concat!(
                "[dependencies.\"github.com/oakcask/foo\"]\n",
                "git = \"materialized/mono\"\n",
                "rev = \"abc123\"\n",
                "subdir = \"packages/foo\"\n",
            ),
        );
        temp.write(
            "main.veln",
            "use foo from \"github.com/oakcask/foo\"\n\npub fn main() -> Int\n  add_one(1)\nend\n",
        );
        temp.write(
            "materialized/mono/packages/foo/veln.toml",
            "[package]\nname = \"github.com/oakcask/foo\"\n\n[lib]\nexports = [\"foo.veln\"]\n",
        );
        temp.write(
            "materialized/mono/packages/foo/foo.veln",
            "pub fn add_one(value: Int) -> Int\n  value + 1\nend\n",
        );

        let project =
            Project::discover(temp.root().to_path_buf(), &[PathBuf::from("main.veln")]).unwrap();
        let (_, diagnostics) = load_surface_module(&project);

        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }

    #[cfg(unix)]
    #[test]
    fn external_path_dependency_without_direct_manifest_does_not_read_sources() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempProject::new("external-path-dependency-missing-manifest");
        temp.write(
            "veln.toml",
            "[dependencies.\"github.com/oakcask/foo\"]\npath = \"vendor/foo\"\n",
        );
        temp.write(
            "main.veln",
            "use foo from \"github.com/oakcask/foo\"\n\npub fn main() -> Int\n  0\nend\n",
        );
        temp.write("vendor/foo/foo.veln", "unreadable source");
        let source = temp.path("vendor/foo/foo.veln");
        let original = fs::metadata(&source).unwrap().permissions();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o000)).unwrap();

        let project =
            Project::discover(temp.root().to_path_buf(), &[PathBuf::from("main.veln")]).unwrap();
        let (_, diagnostics) = load_surface_module(&project);

        fs::set_permissions(&source, original).unwrap();
        if !nix_like_effective_root() {
            assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
            assert_eq!(diagnostics[0].id, "manifest.package_name_mismatch");
            assert!(
                diagnostics[0]
                    .message
                    .contains("dependency package name `<missing>`"),
                "{diagnostics:#?}"
            );
        }
    }

    #[cfg(unix)]
    fn nix_like_effective_root() -> bool {
        fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|status| {
                status
                    .lines()
                    .find(|line| line.starts_with("Uid:"))
                    .and_then(|line| line.split_whitespace().nth(2))
                    .and_then(|uid| uid.parse::<u32>().ok())
            })
            == Some(0)
    }

    #[test]
    fn reachable_resolution_skips_unrelated_annotated_functions() {
        fn resolution_scans(unrelated_count: usize) -> (usize, usize, usize) {
            let mut source = String::from(
                "pub fn main() -> Int\n  helper()\nend\n\nfn helper() -> Int\n  1\nend\n",
            );
            for index in 0..unrelated_count {
                source.push_str(&format!(
                    "\nfn unrelated_{index}(value: Int) -> Int\n  value\nend\n"
                ));
            }
            let module = lower(&source);
            reachability_counters::reset();
            let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
            assert_eq!(reachable.functions.len(), 2);
            reachability_counters::snapshot()
        }

        let base = resolution_scans(0);
        let expanded = resolution_scans(128);

        assert_eq!(
            expanded, base,
            "unrelated annotated functions must not add repeated resolution scans"
        );
    }

    #[test]
    fn reachable_materialization_skips_unrelated_annotated_function_bodies() {
        fn materialized_body_count(unrelated_count: usize) -> usize {
            let mut source = String::from(
                "pub fn main() -> Int\n  helper()\nend\n\nfn helper() -> Int\n  1\nend\n",
            );
            for index in 0..unrelated_count {
                source.push_str(&format!(
                    "\nfn unrelated_{index}(value: Int) -> Int\n  value\nend\n"
                ));
            }
            let module = lower(&source);
            reachability_counters::reset();
            let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
            assert_eq!(reachable.functions.len(), 2);
            reachability_counters::snapshot().2
        }

        assert_eq!(
            materialized_body_count(128),
            materialized_body_count(0),
            "unreachable annotated functions must not be materialized for lowering"
        );
    }

    #[test]
    fn separated_reachable_materialization_skips_unrelated_annotated_function_bodies() {
        fn materialized_body_count(unrelated_count: usize) -> usize {
            let standard = lower(concat!(
                "mod std::prelude\n",
                "pub fn standard_value() -> Int\n",
                "  1\n",
                "end\n",
            ));
            let mut source = String::from(concat!(
                "mod app\n",
                "use std::prelude\n",
                "\n",
                "pub fn main() -> Int\n",
                "  helper() + standard_value()\n",
                "end\n",
                "\n",
                "fn helper() -> Int\n",
                "  1\n",
                "end\n",
            ));
            for index in 0..unrelated_count {
                source.push_str(&format!(
                    "\nfn unrelated_{index}(value: Int) -> Int\n  value\nend\n"
                ));
            }
            let application = lower(&source);
            reachability_counters::reset();
            let reachable = reachable_entry_module_with_standard_cache(
                &standard,
                &application,
                "main",
                FunctionKind::Function,
                &ReachabilityCache::default(),
            );
            assert_eq!(reachable.functions.len(), 3);
            reachability_counters::snapshot().2
        }

        assert_eq!(
            materialized_body_count(128),
            materialized_body_count(0),
            "separated reachable inputs must not materialize unreachable annotated functions"
        );
    }

    #[test]
    fn separated_reachable_inputs_match_combined_resolution_results() {
        let mut standard = lower(concat!(
            "mod std::prelude\n",
            "pub type StandardValue\n",
            "  Present\n",
            "end\n",
            "pub schema Packet\n",
            "  value: Int\n",
            "end\n",
            "pub fn standard_value() -> Int\n",
            "  1\n",
            "end\n",
        ));
        add_payload_codec_for_test(&mut standard);
        let mut application = lower(concat!(
            "mod app\n",
            "use std::prelude\n",
            "\n",
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "\n",
            "fn answer() -> Int effects [Ask]\n",
            "  perform Ask::value()\n",
            "end\n",
            "\n",
            "handler ask(seed: Int) handles Ask\n",
            "  value() => seed\n",
            "end\n",
            "\n",
            "type ApplicationValue\n",
            "  Present\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  value: Int\n",
            "end\n",
            "\n",
            "pub fn exposed = answer\n",
            "\n",
            "pub fn main() -> Int\n",
            "  let handled = handle exposed() with ask(2)\n",
            "  handled + standard_value()\n",
            "end\n",
        ));
        add_payload_codec_for_test(&mut application);
        let mut combined = standard.clone();
        combined.uses.extend(application.uses.clone());
        combined.aliases.extend(application.aliases.clone());
        combined.effects.extend(application.effects.clone());
        combined.handlers.extend(application.handlers.clone());
        combined.types.extend(application.types.clone());
        combined.schemas.extend(application.schemas.clone());
        combined.codecs.extend(application.codecs.clone());
        combined.functions.extend(application.functions.clone());
        combined
            .invalid_names
            .extend(application.invalid_names.clone());
        combined.module = application.module.clone();

        let combined_reachable = reachable_entry_module(&combined, "main", FunctionKind::Function);
        let separated_reachable = reachable_entry_module_with_standard_cache(
            &standard,
            &application,
            "main",
            FunctionKind::Function,
            &ReachabilityCache::default(),
        );

        let combined_functions = reachable_function_names(&combined_reachable);
        let separated_functions = reachable_function_names(&separated_reachable);
        assert_eq!(separated_functions, combined_functions);
        assert_eq!(
            separated_functions,
            vec![
                ("app", "answer"),
                ("app", "main"),
                ("std::prelude", "standard_value"),
            ]
        );
        assert_eq!(
            separated_reachable
                .module
                .as_ref()
                .map(|module| module.name.as_str()),
            Some("app")
        );
        assert_eq!(
            separated_reachable.uses.len(),
            combined_reachable.uses.len()
        );
        assert_eq!(
            separated_reachable.aliases.len(),
            combined_reachable.aliases.len()
        );
        assert_eq!(
            separated_reachable.effects.len(),
            combined_reachable.effects.len()
        );
        assert_eq!(
            separated_reachable.handlers.len(),
            combined_reachable.handlers.len()
        );
        assert_eq!(
            separated_reachable.types.len(),
            combined_reachable.types.len()
        );
        assert_eq!(
            separated_reachable.schemas.len(),
            combined_reachable.schemas.len()
        );
        assert_eq!(
            separated_reachable.codecs.len(),
            combined_reachable.codecs.len()
        );
    }

    #[test]
    fn separated_reachable_inputs_resolve_codec_with_targets() {
        let mut standard = lower(concat!(
            "mod std::prelude\n",
            "pub schema Packet\n",
            "  value: Int\n",
            "end\n",
            "\n",
            "fn decode_payload_packet(input: ByteView, base: ByteOffset) -> DecodeStep<{value: Int}>\n",
            "  NeedMore(NeedEnd)\n",
            "end\n",
        ));
        add_payload_codec_for_test(&mut standard);
        let application = lower(concat!(
            "mod app\n",
            "use std::prelude\n",
            "\n",
            "pub fn main(source: ByteView, base: ByteOffset) -> DecodeStep<{value: Int}>\n",
            "  std::prelude::PayloadCodec(source, base)\n",
            "end\n",
        ));
        let mut combined = standard.clone();
        combined.uses.extend(application.uses.clone());
        combined.aliases.extend(application.aliases.clone());
        combined.effects.extend(application.effects.clone());
        combined.handlers.extend(application.handlers.clone());
        combined.types.extend(application.types.clone());
        combined.schemas.extend(application.schemas.clone());
        combined.codecs.extend(application.codecs.clone());
        combined.functions.extend(application.functions.clone());
        combined
            .invalid_names
            .extend(application.invalid_names.clone());
        combined.module = application.module.clone();

        let combined_reachable = reachable_entry_module(&combined, "main", FunctionKind::Function);
        let separated_reachable = reachable_entry_module_with_standard_cache(
            &standard,
            &application,
            "main",
            FunctionKind::Function,
            &ReachabilityCache::default(),
        );

        let combined_functions = reachable_function_names(&combined_reachable);
        let separated_functions = reachable_function_names(&separated_reachable);
        assert_eq!(separated_functions, combined_functions);
        assert_eq!(
            separated_functions,
            vec![("app", "main"), ("std::prelude", "decode_payload_packet")]
        );
    }

    fn add_payload_codec_for_test(module: &mut SurfaceModule) {
        let schema = module
            .schemas
            .iter()
            .find(|schema| schema.name.as_deref() == Some("Packet"))
            .expect("test standard module should define Packet schema");
        module.codecs.push(CodecDecl {
            node_id: schema.node_id,
            module_name: Some("std::prelude".to_string()),
            visibility: Visibility::Public,
            name: Some("PayloadCodec".to_string()),
            schema: Some("Packet".to_string()),
            directions: vec![CodecDirection::Decode],
            implementations: vec![CodecImplementationClause {
                node_id: schema.node_id,
                direction: CodecDirection::Decode,
                kind: CodecImplementationKind::With {
                    function: Some("decode_payload_packet".to_string()),
                },
                span: schema.span.clone(),
            }],
            span: schema.span.clone(),
        });
    }

    #[test]
    fn project_loading_injects_origin_tagged_standard_prelude_imports() {
        let project = Project {
            root: ".".into(),
            files: vec![SourceFile::new(
                "main.veln",
                "pub fn main() -> Int\n  vec_len([1])\nend\n",
            )],
            manifest: None,
        };

        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        assert!(module.functions.iter().any(|function| {
            function.module_name.as_deref() == Some("std::prelude")
                && function.name.as_deref() == Some("vec_len")
        }));
        assert!(module.uses.iter().any(|use_decl| {
            use_decl.module_name.as_deref() == Some("main")
                && use_decl.name == "std::prelude"
                && use_decl.origin == UseOrigin::ImplicitStandardPrelude
        }));
        assert!(!module.uses.iter().any(|use_decl| {
            use_decl.module_name.as_deref() == Some("std::prelude")
                && use_decl.origin == UseOrigin::ImplicitStandardPrelude
        }));
        assert!(!module.uses.iter().any(|use_decl| {
            use_decl
                .module_name
                .as_deref()
                .is_some_and(|module_name| module_name.starts_with("std::"))
                && use_decl.origin == UseOrigin::ImplicitStandardPrelude
        }));
    }

    #[test]
    fn ordinary_project_does_not_load_http2_modules() {
        let project = Project {
            root: ".".into(),
            files: vec![SourceFile::new(
                "main.veln",
                "pub fn main() -> Int\n  vec_len([1])\nend\n",
            )],
            manifest: None,
        };

        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        assert!(module.functions.iter().all(|function| {
            !function
                .module_name
                .as_deref()
                .is_some_and(|module_name| module_name.starts_with("std::http2::"))
        }));
    }

    #[derive(Debug, PartialEq, Eq)]
    struct StandardInitializationWork {
        loaded_modules: Vec<String>,
        materialized_modules: usize,
        materialized_lowered_bytes: usize,
        prepared_declarations: usize,
    }

    fn load_synthetic_standard(unrelated_count: usize) -> StandardInitializationWork {
        let standard = synthetic_standard_package(unrelated_count);
        let mut diagnostics = Vec::new();
        let mut parts = SurfaceParts::new();
        load_project_sources(
            &single_file_project("pub fn main() -> Int\n  1\nend\n"),
            &mut diagnostics,
            &mut parts,
            None,
        );
        let ((), standard_work) = embedded_standard_counters::observe(|| {
            load_embedded_standard_package_from(&standard, &mut diagnostics, &mut parts, true);
        });
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        check_standard_surface_module(&parts.module);
        standard_initialization_work(
            &standard,
            &parts.module,
            standard_work.materialized_lowered_bytes,
        )
    }

    fn single_file_project(text: &str) -> Project {
        Project {
            root: ".".into(),
            files: vec![SourceFile::new("main.veln", text)],
            manifest: None,
        }
    }

    fn synthetic_standard_package(unrelated_count: usize) -> EmbeddedStandardPackage {
        let modules = synthetic_standard_sources(unrelated_count)
            .into_iter()
            .map(|(path, text)| {
                (
                    standard_module_name(&path),
                    embedded_standard_entry(path, text),
                )
            })
            .collect();
        EmbeddedStandardPackage { modules }
    }

    fn synthetic_standard_sources(unrelated_count: usize) -> [(String, String); 3] {
        [
            (
                "prelude.veln".to_string(),
                concat!(
                    "pub type PreludePayload\n",
                    "  PreludePayload(Int)\n",
                    "end\n",
                    "\n",
                    "pub fn prelude_answer(value: Int) -> Int\n",
                    "  value\n",
                    "end\n",
                )
                .to_string(),
            ),
            (
                "extra.veln".to_string(),
                concat!(
                    "pub fn extra_answer(value: Int) -> Int\n",
                    "  value + 1\n",
                    "end\n",
                )
                .to_string(),
            ),
            (
                "unrelated.veln".to_string(),
                unrelated_annotated_standard_module(unrelated_count),
            ),
        ]
    }

    fn embedded_standard_entry(path: String, text: String) -> EmbeddedStandardModuleEntry {
        EmbeddedStandardModuleEntry {
            lowered: std::borrow::Cow::Owned(lowered_standard_module_bytes(&path, &text)),
            path,
            module: std::sync::OnceLock::new(),
        }
    }

    fn lowered_standard_module_bytes(path: &str, text: &str) -> Vec<u8> {
        let source = SourceFile::new(path, text);
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let mut lowered = veln_ast::lower_surface_ast_with_module_identity(
            &parsed.tree,
            standard_module_name(path),
            source.span(veln_source::TextRange::new(0, 0)),
        );
        for use_decl in &mut lowered.uses {
            let imported = use_decl.name.clone();
            use_decl.name = format!("std::{imported}");
        }
        veln_ast::encode_surface_module(&lowered)
    }

    fn standard_module_name(path: &str) -> String {
        format!("std::{}", path.trim_end_matches(".veln").replace('/', "::"))
    }

    fn unrelated_annotated_standard_module(function_count: usize) -> String {
        let mut text = String::new();
        for index in 0..function_count {
            text.push_str(&format!(
                "pub fn unrelated_{index}(value: Int) -> Int\n  value + {index}\nend\n\n"
            ));
        }
        text
    }

    fn check_standard_surface_module(module: &SurfaceModule) {
        let reusable =
            veln_sema::prepare_current_reusable_standard_surface_module_environment(module);
        let (semantic_diagnostics, checked) =
            veln_sema::check_project_surface_module_with_standard_environment(module, &reusable);
        assert!(semantic_diagnostics.is_empty(), "{semantic_diagnostics:#?}");
        assert!(checked.diagnostics.is_empty(), "{:#?}", checked.diagnostics);
    }

    fn standard_initialization_work(
        standard: &EmbeddedStandardPackage,
        module: &SurfaceModule,
        materialized_lowered_bytes: usize,
    ) -> StandardInitializationWork {
        StandardInitializationWork {
            loaded_modules: loaded_standard_modules(module),
            materialized_modules: materialized_standard_modules(standard),
            materialized_lowered_bytes,
            prepared_declarations: standard_declaration_count(module),
        }
    }

    fn loaded_standard_modules(module: &SurfaceModule) -> Vec<String> {
        let mut modules = module
            .functions
            .iter()
            .filter_map(|function| function.module_name.as_deref())
            .filter(|module_name| module_name.starts_with("std::"))
            .chain(
                module
                    .types
                    .iter()
                    .filter_map(|decl| decl.module_name.as_deref())
                    .filter(|module_name| module_name.starts_with("std::")),
            )
            .map(str::to_string)
            .collect::<Vec<_>>();
        modules.sort_unstable();
        modules.dedup();
        modules
    }

    fn materialized_standard_modules(standard: &EmbeddedStandardPackage) -> usize {
        standard
            .modules
            .values()
            .filter(|entry| entry.module.get().is_some())
            .count()
    }

    fn standard_declaration_count(module: &SurfaceModule) -> usize {
        module
            .functions
            .iter()
            .filter(|decl| is_standard(&decl.module_name))
            .count()
            + module
                .types
                .iter()
                .filter(|decl| is_standard(&decl.module_name))
                .count()
            + module
                .uses
                .iter()
                .filter(|decl| is_standard(&decl.module_name))
                .count()
            + module
                .aliases
                .iter()
                .filter(|decl| is_standard(&decl.module_name))
                .count()
            + module
                .effects
                .iter()
                .filter(|decl| is_standard(&decl.module_name))
                .count()
            + module
                .handlers
                .iter()
                .filter(|decl| is_standard(&decl.module_name))
                .count()
            + module
                .schemas
                .iter()
                .filter(|decl| is_standard(&decl.module_name))
                .count()
            + module
                .codecs
                .iter()
                .filter(|decl| is_standard(&decl.module_name))
                .count()
    }

    fn is_standard(module_name: &Option<String>) -> bool {
        module_name
            .as_deref()
            .is_some_and(|module_name| module_name.starts_with("std::"))
    }

    #[test]
    fn standard_package_loading_keeps_initial_analysis_work_constant_for_unrelated_modules() {
        let base = load_synthetic_standard(0);
        let expanded = load_synthetic_standard(128);

        assert_eq!(base.loaded_modules, vec!["std::prelude".to_string()]);
        assert_eq!(expanded, base);
    }

    #[test]
    fn ordinary_project_loads_private_byte_dependency_through_prelude() {
        let project = Project {
            root: ".".into(),
            files: vec![SourceFile::new(
                "main.veln",
                concat!(
                    "pub fn main() -> Int\n",
                    "  let byte: Byte = Byte(42)\n",
                    "  let chunk: ByteChunk = ByteChunk([byte])\n",
                    "  let offset: ByteOffset = ByteOffset(3)\n",
                    "  let count: ByteCount = byte_chunk_count(chunk)\n",
                    "  let view: ByteView = ByteView(chunk, offset, count)\n",
                    "  match view\n",
                    "    ByteView(ByteChunk(_), ByteOffset(start), ByteCount(length)) => start + length\n",
                    "  end\n",
                    "end\n",
                ),
            )],
            manifest: None,
        };

        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        for name in ["Byte", "ByteChunk", "ByteOffset", "ByteCount", "ByteView"] {
            let owners = module
                .types
                .iter()
                .filter(|type_decl| type_decl.name.as_deref() == Some(name))
                .map(|type_decl| type_decl.module_name.as_deref())
                .collect::<Vec<_>>();
            assert_eq!(owners, [Some("std::bytes")]);

            let aliases = module
                .aliases
                .iter()
                .filter(|alias| {
                    alias.module_name.as_deref() == Some("std::prelude")
                        && alias.name.as_deref() == Some(name)
                })
                .collect::<Vec<_>>();
            assert_eq!(aliases.len(), 1);
            assert_eq!(aliases[0].target, ["bytes", name]);
        }

        let lowered = veln_sema::lower_checked_surface_module(&module);
        assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
        assert!(lowered.core.is_some(), "byte alias usage should lower");
    }

    #[test]
    fn ordinary_project_loads_private_diagnostic_dependency_through_prelude() {
        let project = Project {
            root: ".".into(),
            files: vec![SourceFile::new(
                "main.veln",
                concat!(
                    "pub fn main() -> RuntimeDiagnostic\n",
                    "  let detail: RuntimeDiagnosticDetail = RuntimeValueDiagnostic(list_nil(), \"reason\")\n",
                    "  RuntimeDiagnostic(\"example\", \"message\", detail)\n",
                    "end\n",
                ),
            )],
            manifest: None,
        };

        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        for name in [
            "RuntimeDiagnostic",
            "RuntimeDiagnosticDetail",
            "RuntimeDiagnosticFieldPathSegment",
            "RuntimeByteDiagnosticFacts",
            "RuntimeBytePreview",
            "Http2DiagnosticDetail",
            "HpackDiagnosticDetail",
        ] {
            let owners = module
                .types
                .iter()
                .filter(|type_decl| type_decl.name.as_deref() == Some(name))
                .map(|type_decl| type_decl.module_name.as_deref())
                .collect::<Vec<_>>();
            assert_eq!(owners, [Some("std::diagnostic")]);

            let aliases = module
                .aliases
                .iter()
                .filter(|alias| {
                    alias.module_name.as_deref() == Some("std::prelude")
                        && alias.name.as_deref() == Some(name)
                })
                .collect::<Vec<_>>();
            assert_eq!(aliases.len(), 1);
            assert_eq!(aliases[0].target, ["diagnostic", name]);
        }

        let lowered = veln_sema::lower_checked_surface_module(&module);
        assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
        assert!(
            lowered.core.is_some(),
            "diagnostic alias usage should lower"
        );
    }

    #[test]
    fn explicit_standard_http2_import_loads_only_its_dependency_closure() {
        let project = Project {
            root: ".".into(),
            files: vec![SourceFile::new(
                "main.veln",
                concat!(
                    "use http2::frame from \"std\"\n",
                    "pub fn main(view: ByteView) -> Result<{ length : Int, kind : Int, flags : Int, stream_id : Int, payload : ByteView }, String>\n",
                    "  http2::frame::decode(view)\n",
                    "end\n",
                ),
            )],
            manifest: None,
        };

        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        assert!(module.functions.iter().any(|function| {
            function.module_name.as_deref() == Some("std::http2::frame")
                && function.name.as_deref() == Some("decode")
        }));
        assert!(module.functions.iter().all(|function| {
            function.module_name.as_deref() != Some("std::http2::hpack")
                && function.module_name.as_deref() != Some("std::http2::core")
        }));
    }

    #[test]
    fn explicit_standard_hpack_import_loads_encoder_dependency_closure() {
        let project = Project {
            root: ".".into(),
            files: vec![SourceFile::new(
                "main.veln",
                concat!(
                    "use http2::hpack from \"std\"\n",
                    "pub fn main() -> Result<DynamicTable, String>\n",
                    "  http2::hpack::empty_dynamic_table(64)\n",
                    "end\n",
                ),
            )],
            manifest: None,
        };

        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        for dependency in [
            "std::http2::hpack",
            "std::http2::hpack::header_encoder",
            "std::http2::hpack::header_list_encoder",
            "std::http2::hpack::string_encoder",
        ] {
            assert!(
                module
                    .functions
                    .iter()
                    .any(|function| function.module_name.as_deref() == Some(dependency)),
                "missing HPACK dependency {dependency}"
            );
        }
        assert!(module.functions.iter().all(|function| {
            !matches!(
                function.module_name.as_deref(),
                Some("std::http2::frame")
                    | Some("std::http2::core")
                    | Some("std::http2::diagnostic")
                    | Some("std::http2::hpack::diagnostic")
            )
        }));
    }

    #[test]
    fn private_standard_http2_modules_cannot_be_imported() {
        let project = Project {
            root: ".".into(),
            files: vec![SourceFile::new(
                "main.veln",
                concat!(
                    "use http2::hpack::integer from \"std\"\n",
                    "use http2::core::pending_header_block from \"std\"\n",
                    "pub fn main() -> Int\n",
                    "  0\n",
                    "end\n",
                ),
            )],
            manifest: None,
        };

        let (_, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "module.unexported_import"
                && diagnostic.message.contains("http2::hpack::integer")
        }));
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "module.unexported_import"
                && diagnostic
                    .message
                    .contains("http2::core::pending_header_block")
        }));
    }

    #[test]
    fn private_standard_byte_module_cannot_be_imported() {
        let project = Project {
            root: ".".into(),
            files: vec![SourceFile::new(
                "main.veln",
                concat!(
                    "use bytes from \"std\"\n",
                    "pub fn main() -> Int\n",
                    "  0\n",
                    "end\n",
                ),
            )],
            manifest: None,
        };

        let (_, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "module.unexported_import" && diagnostic.message.contains("bytes")
        }));
    }

    #[test]
    fn private_standard_diagnostic_module_cannot_be_imported() {
        let project = Project {
            root: ".".into(),
            files: vec![SourceFile::new(
                "main.veln",
                concat!(
                    "use diagnostic from \"std\"\n",
                    "pub fn main() -> Int\n",
                    "  0\n",
                    "end\n",
                ),
            )],
            manifest: None,
        };

        let (_, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "module.unexported_import" && diagnostic.message.contains("diagnostic")
        }));
    }

    #[test]
    fn toolchain_standard_project_is_not_loaded_twice() {
        let (module, diagnostics, runtime_standard_parse_lowers, expected_runtime_sources) =
            loaded_toolchain_standard_fixture();
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        assert_eq!(runtime_standard_parse_lowers, expected_runtime_sources);
        assert!(
            module.functions.iter().any(|function| {
                function.module_name.as_deref() == Some("std::http2::core")
                    && function.name.as_deref() == Some("client_stream_id")
            }),
            "functions: {:#?}",
            module
                .functions
                .iter()
                .map(|function| (function.module_name.as_deref(), function.name.as_deref()))
                .collect::<Vec<_>>()
        );
        assert!(
            module.uses.iter().any(|use_decl| {
                use_decl.module_name.as_deref() == Some("std::http2::core_test")
                    && use_decl.name == "std::http2::core"
            }),
            "uses: {:#?}",
            module.uses
        );
        assert_eq!(
            module
                .functions
                .iter()
                .filter(|function| {
                    function.module_name.as_deref() == Some("std::prelude")
                        && function.name.as_deref() == Some("vec_len")
                })
                .count(),
            1
        );
    }

    #[test]
    fn toolchain_standard_project_allows_extra_companion_source() {
        let bundle = veln_stdlib::package_bundle();
        let mut files = bundle
            .files
            .iter()
            .map(|file| SourceFile::new(file.path, file.text))
            .collect::<Vec<_>>();
        files.push(SourceFile::new(
            "prelude.test.veln",
            "test companion() -> ()\nend\n",
        ));
        let project = Project {
            root: ".".into(),
            files,
            manifest: Some(ProjectManifest {
                path: SourcePath::new("veln.toml"),
                source_bytes: Vec::new(),
                package: veln_project::ManifestPackage {
                    fields: vec![ManifestField {
                        key: "name".to_string(),
                        value: veln_stdlib::PACKAGE_NAME.to_string(),
                        key_span: span("veln.toml", 2, 1, 5),
                        value_span: span("veln.toml", 2, 8, 13),
                    }],
                },
                lib: ManifestLib {
                    exports: bundle
                        .exports
                        .iter()
                        .map(|export| ManifestExport {
                            path: (*export).to_string(),
                            path_span: span("veln.toml", 4, 1, 1 + export.len()),
                        })
                        .collect(),
                },
                dependencies: Vec::new(),
                unsupported_sections: Vec::new(),
                tools: Vec::new(),
            }),
        };

        assert!(super::is_toolchain_standard_project(&project));
    }

    #[test]
    fn standard_http2_tests_load_with_private_imports() {
        let (module, diagnostics, _, _) = loaded_toolchain_standard_fixture();

        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        for entry in [
            "receive_frame_dispatch_decodes_headers_with_production_hpack",
            "outbound_request_headers_send_emits_hpack_bytes_and_creates_stream",
            "output_buffer_preserves_successful_send_order",
            "goaway_send_emits_exact_bytes_and_updates_shutdown_immutably",
        ] {
            assert!(
                module.functions.iter().any(|function| {
                    function.module_name.as_deref() == Some("std::http2::core_test")
                        && function.name.as_deref() == Some(entry)
                        && function.kind == FunctionKind::Test
                }),
                "{entry} should load from the standard HTTP/2 core test module"
            );
        }
    }

    #[test]
    fn standard_project_with_manifest_additions_is_reserved_user_package() {
        let mut project = toolchain_standard_project(Vec::new());
        project
            .manifest
            .as_mut()
            .expect("standard project manifest")
            .tools
            .push(ManifestTool {
                name: "extra".to_string(),
                fields: Vec::new(),
            });

        let toolchain_std = super::is_toolchain_standard_project(&project);
        assert!(!toolchain_std);
        let diagnostics = super::validate_reserved_standard_package(&project, toolchain_std);

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "manifest.reserved_standard_package"
                && diagnostic.message == "package name `std` is reserved by the Veln toolchain"
        }));
    }

    fn loaded_toolchain_standard_fixture() -> &'static (SurfaceModule, Vec<Diagnostic>, usize, usize)
    {
        static FIXTURE: OnceLock<(SurfaceModule, Vec<Diagnostic>, usize, usize)> = OnceLock::new();
        FIXTURE.get_or_init(|| {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../veln-stdlib/veln");
            let core_test = fs::read_to_string(root.join("http2/core_test.veln"))
                .expect("standard HTTP/2 core test source should load");
            let project = toolchain_standard_project(vec![SourceFile::new(
                "http2/core_test.veln",
                core_test,
            )]);
            let expected_runtime_sources = project
                .files
                .iter()
                .filter(|source| source.path().as_str().ends_with("_test.veln"))
                .count();
            let ((module, diagnostics), work) =
                embedded_standard_counters::observe(|| load_surface_module(&project));
            (
                module,
                diagnostics,
                work.runtime_standard_parse_lowers,
                expected_runtime_sources,
            )
        })
    }

    fn toolchain_standard_project(additional_files: Vec<SourceFile>) -> Project {
        let bundle = veln_stdlib::package_bundle();
        let mut files = bundle
            .files
            .iter()
            .map(|file| SourceFile::new(file.path, file.text))
            .collect::<Vec<_>>();
        files.extend(additional_files);
        Project {
            root: ".".into(),
            files,
            manifest: Some(parse_manifest_text("veln.toml", bundle.manifest)),
        }
    }

    #[test]
    fn project_standard_calls_lower_through_mangled_veln_functions() {
        let project = Project {
            root: ".".into(),
            files: vec![SourceFile::new(
                "main.veln",
                "pub fn main() -> Int\n  vec_len([1])\nend\n",
            )],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let lowered = veln_sema::lower_project_reachable_surface_module(&reachable);
        assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
        let core = lowered.core.expect("project should lower to core");
        let main = core
            .functions
            .iter()
            .find(|function| function.name == "main")
            .expect("main function");
        assert!(matches!(
            &main.body[0].kind,
            veln_core::CoreStmtKind::Return { expr }
                if matches!(
                    &expr.kind,
                    veln_core::CoreExprKind::Call {
                        target: veln_core::CoreCallTarget::Function(name),
                        ..
                    } if name == "__veln_std$prelude$vec_len"
                )
        ));
        let std_vec_len = core
            .functions
            .iter()
            .find(|function| function.name == "__veln_std$prelude$vec_len")
            .expect("reachable std vec_len body");
        assert!(matches!(
            &std_vec_len.body[0].kind,
            veln_core::CoreStmtKind::Return { expr }
                if matches!(
                    &expr.kind,
                    veln_core::CoreExprKind::Call {
                        target: veln_core::CoreCallTarget::PreludeBuiltin(name),
                        ..
                    } if name == "vec_len"
                )
        ));
    }

    #[test]
    fn test_entry_can_reach_function_callee() {
        let module = lower(concat!(
            "test foo() -> ()\n",
            "  helper()\n",
            "end\n",
            "fn helper() -> ()\n",
            "  ()\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (FunctionKind::Test, Some("foo")),
                (FunctionKind::Function, Some("helper")),
            ]
        );
    }

    #[test]
    fn test_entry_can_reach_function_value_reference() {
        let module = lower(concat!(
            "test foo() -> ()\n",
            "  vec_map([1], stringify)\n",
            "  ()\n",
            "end\n",
            "fn stringify(value: Int) -> String\n",
            "  \"ok\"\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (FunctionKind::Test, Some("foo")),
                (FunctionKind::Function, Some("stringify")),
            ]
        );
    }

    #[test]
    fn test_entry_reaches_same_shape_variadic_function_value_targets() {
        let module = lower(concat!(
            "test foo(callback: fn(String, ...String) -> String) -> ()\n",
            "  callback(\"prefix\", \"a\", \"b\")\n",
            "  ()\n",
            "end\n",
            "fn join(prefix: String, values: ...String) -> String\n",
            "  prefix\n",
            "end\n",
            "fn fixed(prefix: String, value: String) -> String\n",
            "  prefix\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (FunctionKind::Test, Some("foo")),
                (FunctionKind::Function, Some("join")),
            ]
        );
    }

    #[test]
    fn test_entry_does_not_reach_variadic_function_value_targets_for_too_few_args() {
        let module = lower(concat!(
            "test foo(callback: fn(String, ...String) -> String) -> ()\n",
            "  callback()\n",
            "  ()\n",
            "end\n",
            "fn join(prefix: String, values: ...String) -> String\n",
            "  prefix\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(functions, vec![(FunctionKind::Test, Some("foo"))]);
    }

    #[test]
    fn test_entry_conservatively_reaches_opaque_function_value_call_targets() {
        let module = lower(concat!(
            "test foo() -> Bool\n",
            "  invoke(ready)\n",
            "end\n",
            "fn invoke(job: fn() -> Bool) -> Bool\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool\n",
            "  true\n",
            "end\n",
            "fn risky() -> Bool\n",
            "  _\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (FunctionKind::Test, Some("foo")),
                (FunctionKind::Function, Some("invoke")),
                (FunctionKind::Function, Some("ready")),
                (FunctionKind::Function, Some("risky")),
            ]
        );
    }

    #[test]
    fn test_entry_reaches_opaque_function_value_call_targets_with_spaced_type() {
        let module = lower(concat!(
            "test foo() -> Bool\n",
            "  invoke(ready)\n",
            "end\n",
            "fn invoke(job: fn () -> Bool) -> Bool\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool\n",
            "  true\n",
            "end\n",
            "fn risky() -> Bool\n",
            "  _\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (FunctionKind::Test, Some("foo")),
                (FunctionKind::Function, Some("invoke")),
                (FunctionKind::Function, Some("ready")),
                (FunctionKind::Function, Some("risky")),
            ]
        );
    }

    #[test]
    fn test_entry_conservatively_reaches_opaque_local_function_value_call_targets() {
        let module = lower(concat!(
            "test foo() -> Bool\n",
            "  invoke()\n",
            "end\n",
            "fn invoke() -> Bool\n",
            "  let job: fn() -> Bool = ready\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool\n",
            "  true\n",
            "end\n",
            "fn risky() -> Bool\n",
            "  _\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (FunctionKind::Test, Some("foo")),
                (FunctionKind::Function, Some("invoke")),
                (FunctionKind::Function, Some("ready")),
                (FunctionKind::Function, Some("risky")),
            ]
        );
    }

    #[test]
    fn test_entry_can_reach_qualified_function_value_reference() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "app/main_test.veln",
                    concat!(
                        "use app::text\n",
                        "test foo() -> ()\n",
                        "  vec_map([1], app::text::stringify)\n",
                        "  ()\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "app/text.veln",
                    concat!(
                        "pub fn stringify(value: Int) -> String\n",
                        "  \"ok\"\n",
                        "end\n",
                    ),
                ),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (Some("app::main_test"), FunctionKind::Test, Some("foo")),
                (Some("app::text"), FunctionKind::Function, Some("stringify")),
                (
                    Some("std::prelude"),
                    FunctionKind::Function,
                    Some("vec_push")
                ),
                (
                    Some("std::prelude"),
                    FunctionKind::Function,
                    Some("vec_concat")
                ),
                (
                    Some("std::prelude"),
                    FunctionKind::Function,
                    Some("vec_append")
                ),
                (
                    Some("std::prelude"),
                    FunctionKind::Function,
                    Some("vec_map")
                ),
                (
                    Some("std::prelude"),
                    FunctionKind::Function,
                    Some("vec_map_step")
                ),
            ]
        );
    }

    #[test]
    fn companion_test_entry_reaches_qualified_private_target_function() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "math.test.veln",
                    concat!(
                        "use math\n",
                        "test increment_test() -> Int\n",
                        "  math::increment(1)\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "math.veln",
                    concat!(
                        "fn increment(value: Int) -> Int\n",
                        "  value + 1\n",
                        "end\n",
                    ),
                ),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "increment_test", FunctionKind::Test);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert!(
            functions.contains(&(
                Some("math__test_companion"),
                FunctionKind::Test,
                Some("increment_test")
            )),
            "{functions:#?}"
        );
        assert!(
            functions.contains(&(Some("math"), FunctionKind::Function, Some("increment"))),
            "{functions:#?}"
        );
    }

    #[test]
    fn companion_test_entry_keeps_qualified_private_target_handler() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "math.test.veln",
                    concat!(
                        "use math\n",
                        "test handler_test() -> Int\n",
                        "  handle math::compute() with math::ask(41)\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "math.veln",
                    concat!(
                        "effect Ask\n",
                        "  value() -> Int\n",
                        "end\n",
                        "fn provide(offset: Int) -> Int\n",
                        "  offset + 1\n",
                        "end\n",
                        "handler ask(offset: Int) handles Ask\n",
                        "  value() => provide(offset)\n",
                        "end\n",
                        "pub fn compute() -> Int effects [Ask]\n",
                        "  perform Ask::value()\n",
                        "end\n",
                    ),
                ),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "handler_test", FunctionKind::Test);
        let handlers = reachable
            .handlers
            .iter()
            .map(|handler| (handler.module_name.as_deref(), handler.name.as_deref()))
            .collect::<Vec<_>>();
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert!(
            handlers.contains(&(Some("math"), Some("ask"))),
            "{handlers:#?}"
        );
        assert!(
            functions.contains(&(Some("math"), FunctionKind::Function, Some("provide"))),
            "{functions:#?}"
        );
    }

    #[test]
    fn companion_public_alias_cannot_reexport_private_target_function() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "math.test.veln",
                    concat!(
                        "use math\n",
                        "pub fn expose = math::increment\n",
                        "test expose_test() -> Int\n",
                        "  expose(1)\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "math.veln",
                    concat!(
                        "fn increment(value: Int) -> Int\n",
                        "  value + 1\n",
                        "end\n",
                    ),
                ),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.id == "module.companion_public_declaration")
            .unwrap_or_else(|| {
                panic!(
                    "expected companion public declaration diagnostic for alias: {diagnostics:#?}"
                )
            });
        assert_eq!(
            detail_string(diagnostic, "reason"),
            Some("public_function_alias")
        );

        let reachable = reachable_entry_module(&module, "expose_test", FunctionKind::Test);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert!(
            functions.contains(&(
                Some("math__test_companion"),
                FunctionKind::Test,
                Some("expose_test")
            )),
            "{functions:#?}"
        );
        assert!(
            !functions.contains(&(Some("math"), FunctionKind::Function, Some("increment"))),
            "{functions:#?}"
        );
    }

    #[test]
    fn companion_test_entry_does_not_reach_private_target_function_value() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "math.test.veln",
                    concat!(
                        "use math\n",
                        "test increment_value_test() -> Int\n",
                        "  let mapper: fn(Int) -> Int = math::increment\n",
                        "  mapper(1)\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "math.veln",
                    concat!(
                        "fn increment(value: Int) -> Int\n",
                        "  value + 1\n",
                        "end\n",
                    ),
                ),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "increment_value_test", FunctionKind::Test);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert!(
            functions.contains(&(
                Some("math__test_companion"),
                FunctionKind::Test,
                Some("increment_value_test")
            )),
            "{functions:#?}"
        );
        assert!(
            !functions.contains(&(Some("math"), FunctionKind::Function, Some("increment"))),
            "{functions:#?}"
        );
    }

    #[test]
    fn companion_call_does_not_change_production_private_inference_reachability() {
        let target = SourceFile::new(
            "math.veln",
            concat!(
                "fn identity(value)\n",
                "  value\n",
                "end\n",
                "pub fn production() -> Int\n",
                "  identity(_)\n",
                "end\n",
            ),
        );
        let project_without_companion = Project {
            root: ".".into(),
            files: vec![target.clone()],
            manifest: None,
        };
        let project_with_companion = Project {
            root: ".".into(),
            files: vec![
                target,
                SourceFile::new(
                    "math.test.veln",
                    concat!(
                        "use math\n",
                        "test identity_test() -> Int\n",
                        "  math::identity(1)\n",
                        "end\n",
                    ),
                ),
            ],
            manifest: None,
        };

        let (without_companion, without_diagnostics) =
            load_surface_module(&project_without_companion);
        let (with_companion, with_diagnostics) = load_surface_module(&project_with_companion);
        assert!(without_diagnostics.is_empty(), "{without_diagnostics:#?}");
        assert!(with_diagnostics.is_empty(), "{with_diagnostics:#?}");

        let production_without =
            reachable_entry_module(&without_companion, "production", FunctionKind::Function);
        let production_with =
            reachable_entry_module(&with_companion, "production", FunctionKind::Function);
        let without_functions = production_without
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                    function
                        .params
                        .iter()
                        .map(|param| param.ty.as_deref())
                        .collect::<Vec<_>>(),
                    function.return_type.as_deref(),
                )
            })
            .collect::<Vec<_>>();
        let with_functions = production_with
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                    function
                        .params
                        .iter()
                        .map(|param| param.ty.as_deref())
                        .collect::<Vec<_>>(),
                    function.return_type.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(with_functions, without_functions);
    }

    #[test]
    fn run_entry_filters_unreachable_invalid_non_function_names() {
        let module = lower(concat!(
            "fn main() -> Int\n",
            "  1\n",
            "end\n",
            "fn Bad() -> Int\n",
            "  2\n",
            "end\n",
            "type item\n",
            "  value\n",
            "end\n",
            "pub fn Exported = Bad\n",
            "pub type exported = item\n",
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "handler ask(Context: Int) handles Ask\n",
            "  value() => Context\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

        assert!(
            reachable.invalid_names.is_empty(),
            "{:#?}",
            reachable.invalid_names
        );
        assert!(reachable.aliases.is_empty(), "{:#?}", reachable.aliases);
        assert!(reachable.handlers.is_empty(), "{:#?}", reachable.handlers);
    }

    #[test]
    fn run_entry_keeps_invalid_type_names_referenced_by_reachable_signature() {
        let module = lower(concat!(
            "type item\n",
            "  value\n",
            "end\n",
            "fn main() -> item\n",
            "  1\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let invalid_names = reachable
            .invalid_names
            .iter()
            .map(|invalid| invalid.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(invalid_names, vec!["item", "value"]);
    }

    #[test]
    fn run_entry_does_not_reach_invalid_type_from_local_value_spelling() {
        let module = lower(concat!(
            "fn main() -> Int\n",
            "  let item = 1\n",
            "  item\n",
            "end\n",
            "type item\n",
            "  value\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

        assert!(
            reachable.invalid_names.is_empty(),
            "{:#?}",
            reachable.invalid_names
        );
    }

    #[test]
    fn run_entry_does_not_reach_invalid_type_from_record_field_spelling() {
        let module = lower(concat!(
            "fn main() -> {item: Int}\n",
            "  {item: 1}\n",
            "end\n",
            "type item\n",
            "  value\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

        assert!(
            reachable.invalid_names.is_empty(),
            "{:#?}",
            reachable.invalid_names
        );
    }

    #[test]
    fn run_entry_does_not_reach_invalid_alias_from_return_type_spelling() {
        let module = lower(concat!(
            "type Item\n",
            "  Value\n",
            "end\n",
            "fn main() -> Item\n",
            "  Value\n",
            "end\n",
            "fn good() -> Item\n",
            "  Value\n",
            "end\n",
            "pub fn Item = good\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

        assert!(
            reachable.invalid_names.is_empty(),
            "{:#?}",
            reachable.invalid_names
        );
        assert!(reachable.aliases.is_empty(), "{:#?}", reachable.aliases);
    }

    #[test]
    fn run_entry_keeps_reachable_invalid_function_alias_name() {
        let module = lower(concat!(
            "fn main() -> Int\n",
            "  Exported()\n",
            "end\n",
            "fn good() -> Int\n",
            "  1\n",
            "end\n",
            "pub fn Exported = good\n",
            "pub fn Unreachable = good\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let invalid_names = reachable
            .invalid_names
            .iter()
            .map(|invalid| invalid.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(invalid_names, vec!["Exported"]);
        assert!(
            reachable
                .aliases
                .iter()
                .any(|alias| alias.name.as_deref() == Some("Exported"))
        );
        assert!(
            reachable
                .aliases
                .iter()
                .all(|alias| alias.name.as_deref() != Some("Unreachable")),
            "unreachable invalid aliases must not materialize: {:#?}",
            reachable.aliases
        );
    }

    #[test]
    fn run_entry_keeps_invalid_constructor_referenced_by_reachable_expression_path() {
        let module = lower(concat!(
            "fn main() -> Int\n",
            "  value\n",
            "end\n",
            "type item\n",
            "  value\n",
            "end\n",
            "type other\n",
            "  other_value\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let invalid_names = reachable
            .invalid_names
            .iter()
            .map(|invalid| invalid.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(invalid_names, vec!["item", "value"]);
    }

    #[test]
    fn run_entry_keeps_unique_invalid_constructor_call_by_arity() {
        let module = lower(concat!(
            "fn main() -> item\n",
            "  value(1)\n",
            "end\n",
            "type item\n",
            "  value(Int)\n",
            "end\n",
            "type other\n",
            "  value\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let invalid_names = reachable
            .invalid_names
            .iter()
            .map(|invalid| invalid.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(invalid_names, vec!["item", "value"]);
    }

    #[test]
    fn run_entry_does_not_choose_ambiguous_constructor_recovery() {
        let module = lower(concat!(
            "fn main() -> Int\n",
            "  value\n",
            "end\n",
            "type item\n",
            "  value\n",
            "end\n",
            "type other\n",
            "  value\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

        assert!(
            reachable.invalid_names.is_empty(),
            "{:#?}",
            reachable.invalid_names
        );
    }

    #[test]
    fn run_entry_uses_valid_constructor_before_same_spelled_function_recovery() {
        let module = lower(concat!(
            "type Item\n",
            "  Bad\n",
            "end\n",
            "fn main() -> Item\n",
            "  Bad\n",
            "end\n",
            "fn Bad() -> Item\n",
            "  Bad\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

        assert!(
            reachable.invalid_names.is_empty(),
            "{:#?}",
            reachable.invalid_names
        );
    }

    #[test]
    fn run_entry_keeps_invalid_bindings_in_reachable_handler() {
        let module = lower(concat!(
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "fn body() -> Int effects [Ask]\n",
            "  perform Ask::value()\n",
            "end\n",
            "handler ask(Context: Int) handles Ask\n",
            "  value(Result) => Context + Result\n",
            "end\n",
            "fn main() -> Int\n",
            "  handle body() with ask(1)\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let invalid_names = reachable
            .invalid_names
            .iter()
            .map(|invalid| invalid.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(invalid_names, vec!["Context", "Result"]);
        assert_eq!(reachable.handlers.len(), 1, "{:#?}", reachable.handlers);
    }

    #[test]
    fn run_entry_ignores_invalid_bindings_in_unreachable_handler() {
        let module = lower(concat!(
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "handler ask(Context: Int) handles Ask\n",
            "  value(Result) => Context + Result\n",
            "end\n",
            "fn main() -> Int\n",
            "  1\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

        assert!(
            reachable.invalid_names.is_empty(),
            "{:#?}",
            reachable.invalid_names
        );
        assert!(reachable.handlers.is_empty(), "{:#?}", reachable.handlers);
    }

    #[test]
    fn run_entry_keeps_invalid_type_from_reachable_handler_parameter_annotation() {
        let module = lower(concat!(
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "type item\n",
            "  value\n",
            "end\n",
            "fn body() -> Int effects [Ask]\n",
            "  perform Ask::value()\n",
            "end\n",
            "handler ask(seed: item) handles Ask\n",
            "  value() => 1\n",
            "end\n",
            "handler unreachable(seed: other) handles Ask\n",
            "  value() => 2\n",
            "end\n",
            "type other\n",
            "  other_value\n",
            "end\n",
            "fn main() -> Int\n",
            "  handle body() with ask(value)\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let invalid_names = reachable
            .invalid_names
            .iter()
            .map(|invalid| invalid.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(invalid_names, vec!["item", "value"]);
    }

    #[test]
    fn run_entry_keeps_invalid_constructor_from_reachable_handler_clause_expression() {
        let module = lower(concat!(
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "type item\n",
            "  value\n",
            "end\n",
            "fn body() -> Int effects [Ask]\n",
            "  perform Ask::value()\n",
            "end\n",
            "handler ask() handles Ask\n",
            "  value() => value\n",
            "end\n",
            "handler unreachable() handles Ask\n",
            "  value() => other_value\n",
            "end\n",
            "type other\n",
            "  other_value\n",
            "end\n",
            "fn main() -> Int\n",
            "  handle body() with ask()\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let invalid_names = reachable
            .invalid_names
            .iter()
            .map(|invalid| invalid.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(invalid_names, vec!["item", "value"]);
    }

    #[test]
    fn run_entry_keeps_invalid_constructor_from_reachable_handler_match_scrutinee() {
        let module = lower(concat!(
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "type item\n",
            "  value\n",
            "end\n",
            "fn body() -> Int effects [Ask]\n",
            "  perform Ask::value()\n",
            "end\n",
            "handler ask() handles Ask\n",
            "  value() => match value\n",
            "    value => 1\n",
            "  end\n",
            "end\n",
            "handler unreachable() handles Ask\n",
            "  value() => other_value\n",
            "end\n",
            "type other\n",
            "  other_value\n",
            "end\n",
            "fn main() -> Int\n",
            "  handle body() with ask()\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let invalid_names = reachable
            .invalid_names
            .iter()
            .map(|invalid| invalid.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(invalid_names, vec!["item", "value"]);
    }

    #[test]
    fn run_entry_does_not_select_imported_function_recovery() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "app.veln",
                    concat!(
                        "mod app\n",
                        "use helper\n",
                        "fn main() -> Int\n",
                        "  helper::Bad()\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "helper.veln",
                    concat!("mod helper\n", "pub fn Bad() -> Int\n", "  1\n", "end\n"),
                ),
            ],
            manifest: None,
        };
        let (module, _) = load_surface_module(&project);

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

        assert!(
            reachable.invalid_names.is_empty(),
            "{:#?}",
            reachable.invalid_names
        );
    }

    #[test]
    fn run_entry_preserves_qualified_type_references_for_recovery_selection() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "app.veln",
                    concat!(
                        "mod app\n",
                        "use helper\n",
                        "fn main(input: helper::item) -> Int\n",
                        "  1\n",
                        "end\n",
                        "type item\n",
                        "  Value\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "helper.veln",
                    concat!("mod helper\n", "pub type item\n", "  Value\n", "end\n"),
                ),
            ],
            manifest: None,
        };
        let (module, _) = load_surface_module(&project);

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

        assert!(
            reachable.invalid_names.is_empty(),
            "{:#?}",
            reachable.invalid_names
        );
    }

    #[test]
    fn companion_public_declarations_report_stable_reasons() {
        let cases = [
            (
                "public_function",
                concat!("pub fn exposed() -> ()\n", "  ()\n", "end\n"),
            ),
            (
                "public_effect",
                concat!("pub effect Visible\n", "  call() -> ()\n", "end\n"),
            ),
            (
                "public_handler",
                concat!(
                    "effect Ask\n",
                    "  call() -> ()\n",
                    "end\n",
                    "fn provide() -> ()\n",
                    "  ()\n",
                    "end\n",
                    "pub handler visible() handles Ask\n",
                    "  call() => provide()\n",
                    "end\n",
                ),
            ),
            (
                "public_type",
                concat!("pub type Visible\n", "  Case\n", "end\n"),
            ),
            (
                "public_type_variant",
                concat!("type Local\n", "  pub Visible\n", "end\n"),
            ),
            (
                "public_schema",
                concat!(
                    "pub schema Visible\n",
                    "  format binary\n",
                    "  value: UInt8\n",
                    "end\n",
                ),
            ),
            ("public_function_alias", "pub fn visible = math::target\n"),
            ("public_type_alias", "pub type Visible = math::Target\n"),
            ("public_schema_alias", "pub schema Visible = math::Target\n"),
        ];

        for (reason, companion_text) in cases {
            let project = Project {
                root: ".".into(),
                files: vec![
                    SourceFile::new("math.test.veln", companion_text),
                    SourceFile::new("math.veln", "fn target() -> ()\n  ()\nend\n"),
                ],
                manifest: None,
            };

            let (_, diagnostics) = load_surface_module(&project);
            let diagnostic = diagnostics
                .iter()
                .find(|diagnostic| diagnostic.id == "module.companion_public_declaration")
                .unwrap_or_else(|| {
                    panic!(
                        "expected companion public declaration diagnostic for {reason}: {diagnostics:#?}"
                    )
                });

            assert_eq!(
                detail_string(diagnostic, "companion_path"),
                Some("math.test.veln")
            );
            assert_eq!(detail_string(diagnostic, "reason"), Some(reason));
        }
    }

    #[test]
    fn companion_private_declarations_remain_valid() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "math.test.veln",
                    concat!(
                        "fn helper() -> ()\n",
                        "  ()\n",
                        "end\n",
                        "effect Ask\n",
                        "  call() -> ()\n",
                        "end\n",
                        "handler local() handles Ask\n",
                        "  call=helper\n",
                        "end\n",
                        "type Local\n",
                        "  Case\n",
                        "end\n",
                        "schema Packet\n",
                        "  format binary\n",
                        "  value: UInt8\n",
                        "end\n",
                    ),
                ),
                SourceFile::new("math.veln", "fn target() -> ()\n  ()\nend\n"),
            ],
            manifest: None,
        };

        let (_, diagnostics) = load_surface_module(&project);

        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "module.companion_public_declaration"),
            "{diagnostics:#?}"
        );
    }

    #[test]
    fn ordinary_public_declarations_remain_valid() {
        let declaration = concat!(
            "pub fn exposed() -> ()\n",
            "  ()\n",
            "end\n",
            "pub effect Ask\n",
            "  call() -> ()\n",
            "end\n",
            "pub handler visible() handles Ask\n",
            "  call=exposed\n",
            "end\n",
            "pub type Visible\n",
            "  pub Case\n",
            "end\n",
            "pub schema Packet\n",
            "  format binary\n",
            "  value: UInt8\n",
            "end\n",
            "pub fn alias = math::exposed\n",
            "pub type Alias = math::Visible\n",
            "pub schema PacketAlias = math::Packet\n",
        );
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new("math.veln", declaration),
                SourceFile::new("math_test.veln", declaration),
            ],
            manifest: None,
        };

        let (_, diagnostics) = load_surface_module(&project);

        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "module.companion_public_declaration"),
            "{diagnostics:#?}"
        );
    }

    #[test]
    fn run_entry_keeps_schema_decode_expression_in_entry_function() {
        let project = Project {
            root: ".".into(),
            files: vec![SourceFile::new(
                "main.veln",
                concat!(
                    "schema PacketWire\n",
                    "  format binary\n",
                    "  length: UInt8\n",
                    "end\n",
                    "\n",
                    "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
                    "  decode PacketWire from view at base\n",
                    "end\n",
                ),
            )],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![(Some("main"), FunctionKind::Function, Some("main"))]
        );
    }

    #[test]
    fn run_entry_keeps_schema_encode_expression_in_entry_function() {
        let project = Project {
            root: ".".into(),
            files: vec![SourceFile::new(
                "main.veln",
                concat!(
                    "schema PacketWire\n",
                    "  format binary\n",
                    "  length: UInt8\n",
                    "end\n",
                    "\n",
                    "pub fn main(packet: {length: Int}) -> Result<ByteChunk, EncodeError>\n",
                    "  encode PacketWire from packet\n",
                    "end\n",
                ),
            )],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![(Some("main"), FunctionKind::Function, Some("main"))]
        );
    }

    #[test]
    fn run_entry_can_reach_contract_helper() {
        let module = lower(concat!(
            "fn positive(value: Int) -> Bool\n",
            "  value > 0\n",
            "end\n",
            "pub fn main(value: Int) -> output: Int\n",
            "  ensure positive(output)\n",
            "  value\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (FunctionKind::Function, Some("positive")),
                (FunctionKind::Function, Some("main")),
            ]
        );
    }

    #[test]
    fn run_entry_can_reach_contract_function_value() {
        let module = lower(concat!(
            "fn accepts(job: fn() -> Bool) -> Bool\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool\n",
            "  true\n",
            "end\n",
            "pub fn main() -> ()\n",
            "  require accepts(ready)\n",
            "  ()\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (FunctionKind::Function, Some("accepts")),
                (FunctionKind::Function, Some("ready")),
                (FunctionKind::Function, Some("main")),
            ]
        );
    }

    #[test]
    fn run_entry_can_reach_qualified_contract_helper() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "app/main.veln",
                    concat!(
                        "use app::rules\n",
                        "pub fn main(value: Int) -> output: Int\n",
                        "  ensure app::rules::positive(output)\n",
                        "  value\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "app/rules.veln",
                    concat!(
                        "pub fn positive(value: Int) -> Bool\n",
                        "  value > 0\n",
                        "end\n",
                    ),
                ),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (Some("app::main"), FunctionKind::Function, Some("main")),
                (Some("app::rules"), FunctionKind::Function, Some("positive")),
            ]
        );
    }

    #[test]
    fn run_entry_can_reach_imported_qualified_call() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "app/main.veln",
                    concat!(
                        "use app::util\n",
                        "pub fn main() -> Int\n",
                        "  app::util::value()\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "app/util.veln",
                    concat!("pub fn value() -> Int\n", "  1\n", "end\n",),
                ),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (Some("app::main"), FunctionKind::Function, Some("main")),
                (Some("app::util"), FunctionKind::Function, Some("value")),
            ]
        );
    }

    #[test]
    fn run_entry_can_reach_imported_alias_target() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "app/main.veln",
                    concat!(
                        "use app::api\n",
                        "pub fn main() -> Int\n",
                        "  app::api::twice(21)\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "app/api.veln",
                    concat!("use app::impl\n", "pub fn twice = app::impl::double\n",),
                ),
                SourceFile::new(
                    "app/impl.veln",
                    concat!(
                        "fn double(value: Int) -> Int\n",
                        "  value + value\n",
                        "end\n",
                    ),
                ),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (Some("app::main"), FunctionKind::Function, Some("main")),
                (Some("app::impl"), FunctionKind::Function, Some("double")),
            ]
        );
    }

    #[test]
    fn run_entry_can_reach_qualified_contract_function_value() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "app/main.veln",
                    concat!(
                        "use app::rules\n",
                        "fn accepts(job: fn() -> Bool) -> Bool\n",
                        "  job()\n",
                        "end\n",
                        "pub fn main() -> ()\n",
                        "  require accepts(app::rules::ready)\n",
                        "  ()\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "app/rules.veln",
                    concat!("pub fn ready() -> Bool\n", "  true\n", "end\n",),
                ),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (Some("app::main"), FunctionKind::Function, Some("accepts")),
                (Some("app::main"), FunctionKind::Function, Some("main")),
                (Some("app::rules"), FunctionKind::Function, Some("ready")),
            ]
        );
    }

    #[test]
    fn imported_reachability_keeps_module_specific_function_names() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "app/main.veln",
                    concat!(
                        "use app::util\n",
                        "fn value() -> Int\n",
                        "  _\n",
                        "end\n",
                        "pub fn main() -> Int\n",
                        "  app::util::value()\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "app/util.veln",
                    concat!("pub fn value() -> Int\n", "  1\n", "end\n",),
                ),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (Some("app::main"), FunctionKind::Function, Some("main")),
                (Some("app::util"), FunctionKind::Function, Some("value")),
            ]
        );
    }

    #[test]
    fn bare_reachability_keeps_current_module_function_names() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "app/main.veln",
                    concat!(
                        "fn value() -> Int\n",
                        "  1\n",
                        "end\n",
                        "pub fn main() -> Int\n",
                        "  value()\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "app/other.veln",
                    concat!("fn value() -> Int\n", "  _\n", "end\n",),
                ),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (Some("app::main"), FunctionKind::Function, Some("value")),
                (Some("app::main"), FunctionKind::Function, Some("main")),
            ]
        );
    }

    #[test]
    fn local_binding_shadowing_function_name_does_not_reach_function() {
        let module = lower(concat!(
            "fn helper() -> Int\n",
            "  _\n",
            "end\n",
            "pub fn main() -> Int\n",
            "  let helper = 1\n",
            "  helper\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(functions, vec![(FunctionKind::Function, Some("main"))]);
    }

    #[test]
    fn match_binding_shadowing_function_name_does_not_reach_function() {
        let module = lower(concat!(
            "fn helper() -> Int\n",
            "  _\n",
            "end\n",
            "pub fn main(value: Option<Int>) -> Int\n",
            "  match value\n",
            "    Some(helper) => helper\n",
            "    None => 0\n",
            "  end\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(functions, vec![(FunctionKind::Function, Some("main"))]);
    }

    #[test]
    fn run_entry_does_not_reach_qualified_call_without_import_alias() {
        let module = lower(concat!(
            "pub fn main() -> Int\n",
            "  util::value()\n",
            "end\n",
            "fn value() -> Int\n",
            "  _\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(functions, vec![(FunctionKind::Function, Some("main"))]);
    }

    #[test]
    fn contract_reachability_ignores_function_names_inside_strings() {
        let module = lower(concat!(
            "fn positive(value: Int) -> Bool\n",
            "  value > 0\n",
            "end\n",
            "pub fn main() -> output: String\n",
            "  ensure \"positive(\" == output\n",
            "  \"positive(\"\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(functions, vec![(FunctionKind::Function, Some("main"))]);
    }

    #[test]
    fn run_entry_does_not_include_tests() {
        let module = lower(concat!(
            "test helper() -> ()\n",
            "  ()\n",
            "end\n",
            "fn foo() -> ()\n",
            "  ()\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "foo", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(functions, vec![(FunctionKind::Function, Some("foo"))]);
    }

    #[test]
    fn run_entry_reaches_spawn_with_context_function_value() {
        let module = lower(concat!(
            "fn combine(context: {payload: String, suffix: String}) -> String effects [concurrency]\n",
            "  suffix(context.suffix)\n",
            "end\n",
            "fn suffix(value: String) -> String\n",
            "  value\n",
            "end\n",
            "pub fn main() -> Task<String> effects [concurrency]\n",
            "  task::spawn_with(combine, {payload: \"body\", suffix: \"tail\"})\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (FunctionKind::Function, Some("combine")),
                (FunctionKind::Function, Some("suffix")),
                (FunctionKind::Function, Some("main")),
            ]
        );
    }

    #[test]
    fn modules_manifest_section_is_rejected() {
        let source = SourceFile::new("src/main.veln", "fn main() -> ()\n  ()\nend\n");
        let project = Project {
            root: ".".into(),
            files: vec![source],
            manifest: Some(ProjectManifest {
                path: SourcePath::new("veln.toml"),
                source_bytes: Vec::new(),
                package: Default::default(),
                lib: ManifestLib {
                    exports: Vec::new(),
                },
                dependencies: Vec::new(),
                unsupported_sections: vec![ManifestUnsupportedSection {
                    name: "modules".to_string(),
                    span: span("veln.toml", 1, 2, 9),
                }],
                tools: Vec::new(),
            }),
        };

        let (module, diagnostics) = load_surface_module(&project);

        assert_eq!(module.module.as_ref().unwrap().name, "src::main");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].id, "manifest.unsupported_section");
        assert_eq!(
            diagnostics[0].message,
            "`[modules]` is not supported; use `[lib].exports` for public source files"
        );
    }

    #[test]
    fn source_mod_declaration_reports_module_diagnostic() {
        let source = SourceFile::new(
            "src/main.veln",
            "mod app.main\nfn main() -> ()\n  ()\nend\n",
        );
        let project = Project {
            root: ".".into(),
            files: vec![source],
            manifest: None,
        };

        let (_, diagnostics) = load_surface_module(&project);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].id, "module.source_mod");
        assert_eq!(
            diagnostics[0].message,
            "source `mod` declarations are not supported"
        );
    }

    #[test]
    fn selected_manifest_export_is_accepted() {
        let source = SourceFile::new("src/main.veln", "fn main() -> ()\n  ()\nend\n");
        let project = Project {
            root: ".".into(),
            files: vec![source],
            manifest: Some(ProjectManifest {
                path: SourcePath::new("veln.toml"),
                source_bytes: Vec::new(),
                package: Default::default(),
                lib: ManifestLib {
                    exports: vec![ManifestExport {
                        path: "src/main.veln".to_string(),
                        path_span: span("veln.toml", 2, 13, 26),
                    }],
                },
                dependencies: Vec::new(),
                unsupported_sections: Vec::new(),
                tools: Vec::new(),
            }),
        };

        let (module, diagnostics) = load_surface_module(&project);

        assert_eq!(module.module.as_ref().unwrap().name, "src::main");
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "manifest.invalid_export"),
            "{diagnostics:#?}"
        );
    }

    #[test]
    fn selected_manifest_export_with_parse_errors_is_still_selected() {
        let source = SourceFile::new("main.veln", "fn main() -> ()\n");
        let project = Project {
            root: ".".into(),
            files: vec![source],
            manifest: Some(ProjectManifest {
                path: SourcePath::new("veln.toml"),
                source_bytes: Vec::new(),
                package: Default::default(),
                lib: ManifestLib {
                    exports: vec![ManifestExport {
                        path: "main.veln".to_string(),
                        path_span: span("veln.toml", 2, 13, 22),
                    }],
                },
                dependencies: Vec::new(),
                unsupported_sections: Vec::new(),
                tools: Vec::new(),
            }),
        };

        let (_, diagnostics) = load_surface_module(&project);

        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "manifest.unselected_export"),
            "{diagnostics:#?}"
        );
    }

    #[test]
    fn manifest_export_validation_preserves_manifest_order_and_first_duplicate_origin() {
        let source = SourceFile::new("src/main.veln", "fn main() -> ()\n  ()\nend\n");
        let project = Project {
            root: ".".into(),
            files: vec![source],
            manifest: Some(ProjectManifest {
                path: SourcePath::new("veln.toml"),
                source_bytes: Vec::new(),
                package: Default::default(),
                lib: ManifestLib {
                    exports: vec![
                        ManifestExport {
                            path: "../outside.veln".to_string(),
                            path_span: span("veln.toml", 2, 4, 21),
                        },
                        ManifestExport {
                            path: "missing.veln".to_string(),
                            path_span: span("veln.toml", 3, 4, 18),
                        },
                        ManifestExport {
                            path: "src/main.veln".to_string(),
                            path_span: span("veln.toml", 4, 4, 19),
                        },
                        ManifestExport {
                            path: "./src/main.veln".to_string(),
                            path_span: span("veln.toml", 5, 4, 21),
                        },
                    ],
                },
                dependencies: Vec::new(),
                unsupported_sections: Vec::new(),
                tools: Vec::new(),
            }),
        };

        let diagnostics = validate_manifest_exports(&project);

        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "manifest.invalid_export",
                "manifest.missing_export",
                "manifest.duplicate_export",
            ]
        );
        assert_eq!(
            diagnostics[2].message,
            "manifest export `./src/main.veln` duplicates module export `src::main`"
        );
        assert_eq!(diagnostics[2].related.len(), 1);
    }

    #[test]
    fn companion_manifest_export_reports_boundary_before_selection_checks() {
        let root = env::temp_dir().join(format!(
            "veln-surface-companion-export-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(root.join("math.test.veln"), "test companion() -> ()\nend\n")
            .expect("companion source should be written");
        let source = SourceFile::new("math.veln", "pub fn value() -> Int\n  1\nend\n");
        let project = Project {
            root: root.clone(),
            files: vec![source],
            manifest: Some(ProjectManifest {
                path: SourcePath::new("veln.toml"),
                source_bytes: Vec::new(),
                package: Default::default(),
                lib: ManifestLib {
                    exports: vec![
                        ManifestExport {
                            path: "math.test.veln".to_string(),
                            path_span: span("veln.toml", 3, 4, 20),
                        },
                        ManifestExport {
                            path: "missing.test.veln".to_string(),
                            path_span: span("veln.toml", 4, 4, 23),
                        },
                    ],
                },
                dependencies: Vec::new(),
                unsupported_sections: Vec::new(),
                tools: Vec::new(),
            }),
        };

        let (_, diagnostics) = load_surface_module(&project);
        let _ = fs::remove_dir_all(&root);

        let invalid_exports = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "manifest.invalid_export")
            .collect::<Vec<_>>();
        assert_eq!(invalid_exports.len(), 2, "{diagnostics:#?}");
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "manifest.unselected_export"),
            "{diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "manifest.missing_export"),
            "{diagnostics:#?}"
        );
        assert_eq!(
            invalid_exports[0].message,
            "manifest export `math.test.veln` is invalid: export names a test companion"
        );
        assert_eq!(
            detail_string(invalid_exports[0], "field"),
            Some("lib.exports")
        );
        assert_eq!(
            detail_string(invalid_exports[0], "source_path"),
            Some("math.test.veln")
        );
        assert_eq!(
            detail_string(invalid_exports[0], "companion_path"),
            Some("math.test.veln")
        );
        assert_eq!(
            detail_string(invalid_exports[0], "reason"),
            Some("test_companion")
        );
    }

    #[test]
    fn unselected_manifest_export_reports_diagnostic() {
        let root = env::temp_dir().join(format!(
            "veln-surface-unselected-export-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).expect("test root should be created");
        fs::write(root.join("src/other.veln"), "fn other() -> ()\n  ()\nend\n")
            .expect("unselected source should be written");
        let source = SourceFile::new("src/main.veln", "fn main() -> ()\n  ()\nend\n");
        let project = Project {
            root: root.clone(),
            files: vec![source],
            manifest: Some(ProjectManifest {
                path: SourcePath::new("veln.toml"),
                source_bytes: Vec::new(),
                package: Default::default(),
                lib: ManifestLib {
                    exports: vec![ManifestExport {
                        path: "src/other.veln".to_string(),
                        path_span: span("veln.toml", 2, 13, 27),
                    }],
                },
                dependencies: Vec::new(),
                unsupported_sections: Vec::new(),
                tools: Vec::new(),
            }),
        };

        let (_, diagnostics) = load_surface_module(&project);
        let _ = fs::remove_dir_all(&root);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].id, "manifest.unselected_export");
        assert_eq!(
            diagnostics[0].message,
            "manifest export `src/other.veln` has no matching selected source file"
        );
    }

    fn span(file: &str, line: usize, start_column: usize, end_column: usize) -> SourceSpan {
        SourceSpan {
            file: SourcePath::new(file),
            start: LineCol {
                line,
                column: start_column,
                offset: 0,
            },
            end: LineCol {
                line,
                column: end_column,
                offset: 0,
            },
        }
    }

    fn detail_string<'a>(
        diagnostic: &'a veln_diagnostics::Diagnostic,
        key: &str,
    ) -> Option<&'a str> {
        let veln_diagnostics::JsonValue::Object(entries) = &diagnostic.details else {
            return None;
        };
        entries.iter().find_map(|(entry_key, value)| {
            if entry_key == key
                && let veln_diagnostics::JsonValue::String(value) = value
            {
                Some(value.as_str())
            } else {
                None
            }
        })
    }
}
