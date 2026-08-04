use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Component, Path};
use std::sync::OnceLock;

use veln_ast::{
    CodecImplementationKind, Expr, ExprKind, Function, FunctionKind, Pattern, PatternKind,
    PublicAliasKind, SurfaceModule, UseDecl, Visibility, lower_surface_ast,
    lower_surface_ast_with_module_identity,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_project::{
    ManifestDependencySelectorKind, ManifestField, Project, ProjectManifest,
    classify_companion_source,
};
use veln_source::{SourceFile, SourcePath, SourceSpan, TextRange};
use veln_syntax::{TokenKind, lex, parse};

use crate::diagnostics::parse_diagnostic_to_envelope;

#[derive(Clone)]
pub(crate) struct LoadedSurfaceModules {
    pub(crate) combined: SurfaceModule,
    pub(crate) application: SurfaceModule,
    pub(crate) selected_standard_module_names: BTreeSet<String>,
}

pub fn load_surface_module(project: &Project) -> (SurfaceModule, Vec<Diagnostic>) {
    let (modules, diagnostics) = load_surface_modules_with_combined(project, true);
    (modules.combined, diagnostics)
}

pub(crate) fn load_surface_modules(project: &Project) -> (LoadedSurfaceModules, Vec<Diagnostic>) {
    load_surface_modules_with_combined(project, false)
}

fn load_surface_modules_with_combined(
    project: &Project,
    include_combined: bool,
) -> (LoadedSurfaceModules, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut parts = SurfaceParts::new();
    let toolchain_std = is_toolchain_standard_project(project);

    load_project_sources(
        project,
        &mut diagnostics,
        &mut parts,
        toolchain_std.then_some(veln_stdlib::PACKAGE_NAME),
    );
    diagnostics.extend(validate_manifest_exports(project));
    diagnostics.extend(validate_manifest_dependencies(project));
    diagnostics.extend(validate_companion_sources(project));
    diagnostics.extend(validate_companion_public_declarations(&parts.module));
    diagnostics.extend(validate_reserved_standard_package(project, toolchain_std));
    load_external_dependencies(project, &mut diagnostics, &mut parts);
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
    text: String,
    module: OnceLock<EmbeddedStandardModule>,
}

struct EmbeddedStandardModule {
    parts: SurfaceParts,
    diagnostics: Vec<Diagnostic>,
}

impl EmbeddedStandardModuleEntry {
    fn module(&self) -> &EmbeddedStandardModule {
        self.module.get_or_init(|| {
            let project = Project {
                root: ".".into(),
                files: vec![SourceFile::new(self.path.as_str(), self.text.as_str())],
                manifest: None,
            };
            let mut diagnostics = Vec::new();
            let mut parts = SurfaceParts::new();
            load_project_sources(
                &project,
                &mut diagnostics,
                &mut parts,
                Some(veln_stdlib::PACKAGE_NAME),
            );
            EmbeddedStandardModule { parts, diagnostics }
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
        let parsed = parse(source);
        diagnostics.extend(parsed.diagnostics.iter().map(parse_diagnostic_to_envelope));
        if !parsed.diagnostics.is_empty() {
            continue;
        }
        process_parsed_source(source, &parsed.tree, diagnostics, parts, package);
    }
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
        load_external_dependency_package(project, package, &use_decl, diagnostics, parts);
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
            .files
            .iter()
            .filter_map(|file| {
                embedded_standard_module_name_from_path(file.path).map(|module_name| {
                    (
                        module_name,
                        EmbeddedStandardModuleEntry {
                            path: file.path.to_string(),
                            text: file.text.to_string(),
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
    package: &str,
    use_decl: &UseDecl,
    diagnostics: &mut Vec<Diagnostic>,
    parts: &mut SurfaceParts,
) {
    let Some((dependency_project, dependency)) =
        load_external_dependency_project(project, package, use_decl, diagnostics)
    else {
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
    let Some(path_field) = &dependency.path else {
        diagnostics.push(unavailable_external_package_diagnostic(use_decl));
        return None;
    };

    let dependency_root = project.root.join(&path_field.value);
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

pub fn derive_source_module_path(source: &SourceFile) -> Result<String, Box<Diagnostic>> {
    let path = source.path().as_str();
    if let Some(module_name) = derive_doctest_module_path(path) {
        return Ok(module_name);
    }
    if let Some(companion) = classify_companion_source(path) {
        if companion.chained {
            let Some(without_extension) = path.strip_suffix(".veln") else {
                return Err(Box::new(invalid_source_module_path_diagnostic(
                    source,
                    path,
                    "source module files must use the `.veln` extension",
                )));
            };
            let segments = without_extension
                .split('/')
                .map(|segment| {
                    let sanitized = segment
                        .chars()
                        .map(|ch| {
                            if ch.is_ascii_alphanumeric() || ch == '_' {
                                ch
                            } else {
                                '_'
                            }
                        })
                        .collect::<String>();
                    format!("{sanitized}__chained_companion")
                })
                .collect::<Vec<_>>();
            return Ok(segments.join("::"));
        }
        let Some(target_stem) = companion.target_path.strip_suffix(".veln") else {
            return Err(Box::new(invalid_source_module_path_diagnostic(
                source,
                path,
                "source module files must use the `.veln` extension",
            )));
        };
        let mut segments = Vec::new();
        for segment in target_stem.split('/') {
            if is_module_identifier(segment) {
                segments.push(segment.to_string());
            } else {
                return Err(Box::new(invalid_source_module_path_diagnostic(
                    source,
                    segment,
                    "source path segment cannot be used as a module identifier",
                )));
            }
        }
        let Some(last) = segments.last_mut() else {
            return Err(Box::new(invalid_source_module_path_diagnostic(
                source,
                path,
                "source path segment cannot be used as a module identifier",
            )));
        };
        *last = format!("{last}__test_companion");
        return Ok(segments.join("::"));
    }
    let Some(without_extension) = path.strip_suffix(".veln") else {
        return Err(Box::new(invalid_source_module_path_diagnostic(
            source,
            path,
            "source module files must use the `.veln` extension",
        )));
    };
    let mut segments = Vec::new();
    for segment in without_extension.split('/') {
        if is_module_identifier(segment) {
            segments.push(segment);
        } else {
            return Err(Box::new(invalid_source_module_path_diagnostic(
                source,
                segment,
                "source path segment cannot be used as a module identifier",
            )));
        }
    }
    Ok(segments.join("::"))
}

fn derive_doctest_module_path(path: &str) -> Option<String> {
    let (source_path, _) = path.split_once("#doctest-")?;
    let source_stem = source_path.strip_suffix(".veln")?;
    let mut segments = Vec::new();
    for segment in source_stem.split('/') {
        if is_module_identifier(segment) {
            segments.push(segment.to_string());
        } else {
            return None;
        }
    }
    Some(segments.join("::"))
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
    for function in &module.functions {
        if function.visibility == Visibility::Public
            && let Some(companion_path) = companion_path_for_span(&function.span)
        {
            diagnostics.push(companion_public_declaration_diagnostic(
                function.span.clone(),
                companion_path,
                "public_function",
                "function",
                function.name.as_deref(),
            ));
        }
    }
    for effect in &module.effects {
        if effect.visibility == Visibility::Public
            && let Some(companion_path) = companion_path_for_span(&effect.span)
        {
            diagnostics.push(companion_public_declaration_diagnostic(
                effect.span.clone(),
                companion_path,
                "public_effect",
                "effect",
                effect.name.as_deref(),
            ));
        }
    }
    for handler in &module.handlers {
        if handler.visibility == Visibility::Public
            && let Some(companion_path) = companion_path_for_span(&handler.span)
        {
            diagnostics.push(companion_public_declaration_diagnostic(
                handler.span.clone(),
                companion_path,
                "public_handler",
                "handler",
                handler.name.as_deref(),
            ));
        }
    }
    for ty in &module.types {
        if ty.visibility == Visibility::Public
            && let Some(companion_path) = companion_path_for_span(&ty.span)
        {
            diagnostics.push(companion_public_declaration_diagnostic(
                ty.span.clone(),
                companion_path,
                "public_type",
                "type",
                ty.name.as_deref(),
            ));
        }
        for variant in &ty.variants {
            if variant.visibility == Visibility::Public
                && let Some(companion_path) = companion_path_for_span(&variant.span)
            {
                diagnostics.push(companion_public_declaration_diagnostic(
                    variant.span.clone(),
                    companion_path,
                    "public_type_variant",
                    "type variant",
                    variant.name.as_deref(),
                ));
            }
        }
    }
    for schema in &module.schemas {
        if schema.visibility == Visibility::Public
            && let Some(companion_path) = companion_path_for_span(&schema.span)
        {
            diagnostics.push(companion_public_declaration_diagnostic(
                schema.span.clone(),
                companion_path,
                "public_schema",
                "schema",
                schema.name.as_deref(),
            ));
        }
    }
    for alias in &module.aliases {
        if let Some(companion_path) = companion_path_for_span(&alias.span) {
            let (reason, declaration_kind) = match alias.kind {
                PublicAliasKind::Function => ("public_function_alias", "function alias"),
                PublicAliasKind::Type => ("public_type_alias", "type alias"),
                PublicAliasKind::Schema => ("public_schema_alias", "schema alias"),
            };
            diagnostics.push(companion_public_declaration_diagnostic(
                alias.span.clone(),
                companion_path,
                reason,
                declaration_kind,
                alias.name.as_deref(),
            ));
        }
    }
    diagnostics
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

fn is_module_identifier(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn invalid_source_module_path_diagnostic(
    source: &SourceFile,
    segment: &str,
    message: &'static str,
) -> Diagnostic {
    Diagnostic::new(
        "module.invalid_source_path",
        Severity::Error,
        DiagnosticKind::Module,
        format!("{message}: `{segment}`"),
        Some(source.span(veln_source::TextRange::new(0, 0))),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("module_identity")),
            ("source_path", JsonValue::string(source.path().as_str())),
            ("segment", JsonValue::string(segment)),
        ]),
    )
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
        let normalized_path = SourcePath::new(export.path.clone());
        let path = normalized_path.as_str();
        if export.path.contains("::") {
            diagnostics.push(invalid_manifest_export_path_diagnostic(
                &export.path_span,
                &export.path,
                "module paths are not valid manifest exports; use a package-relative source file path",
            ));
            continue;
        }
        if !is_package_relative_path(path) {
            diagnostics.push(invalid_manifest_export_path_diagnostic(
                &export.path_span,
                &export.path,
                "manifest exports must stay inside the package",
            ));
            continue;
        }
        if !path.ends_with(".veln") {
            diagnostics.push(invalid_manifest_export_path_diagnostic(
                &export.path_span,
                &export.path,
                "manifest exports must name `.veln` source files",
            ));
            continue;
        }
        if let Some(companion) = classify_companion_source(path) {
            diagnostics.push(companion_manifest_export_diagnostic(
                &export.path_span,
                &export.path,
                &companion.companion_path,
            ));
            continue;
        }
        let export_source = SourceFile::new(path, "");
        let module_name = match derive_source_module_path(&export_source) {
            Ok(module_name) => module_name,
            Err(_) => {
                diagnostics.push(invalid_manifest_export_path_diagnostic(
                    &export.path_span,
                    &export.path,
                    "manifest export path does not derive a valid module path",
                ));
                continue;
            }
        };
        if !project
            .files
            .iter()
            .any(|source| source.path().as_str() == path)
        {
            if project.root.join(path).is_file() {
                diagnostics.push(unselected_manifest_export_diagnostic(
                    &export.path_span,
                    &export.path,
                ));
            } else {
                diagnostics.push(missing_manifest_export_diagnostic(
                    &export.path_span,
                    &export.path,
                ));
            }
            continue;
        }
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
    function_targets: OnceCell<ReachabilityIndex>,
    direct_callees: RefCell<HashMap<ReachableFunction, Vec<ReachableFunction>>>,
}

struct ReachabilityIndex {
    function_targets: FunctionTargetIndex,
    functions_by_name: HashMap<(FunctionKind, String), Vec<usize>>,
    functions_by_qualified_name: HashMap<(FunctionKind, String, String), Vec<usize>>,
}

impl ReachabilityIndex {
    fn new(module: &SurfaceModule, function_targets: Vec<FunctionTarget>) -> Self {
        let mut functions_by_name = HashMap::<(FunctionKind, String), Vec<usize>>::new();
        let mut functions_by_qualified_name =
            HashMap::<(FunctionKind, String, String), Vec<usize>>::new();
        for (index, function) in module.functions.iter().enumerate() {
            let Some(name) = &function.name else {
                continue;
            };
            functions_by_name
                .entry((function.kind, name.clone()))
                .or_default()
                .push(index);
            if let Some(module_name) = &function.module_name {
                functions_by_qualified_name
                    .entry((function.kind, module_name.clone(), name.clone()))
                    .or_default()
                    .push(index);
            }
        }
        Self {
            function_targets: FunctionTargetIndex::new(function_targets),
            functions_by_name,
            functions_by_qualified_name,
        }
    }

    fn function_indices(&self, key: &ReachableFunction) -> &[usize] {
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
    }

    pub(super) fn reset() {
        FUNCTION_LOOKUP_SCANS.set(0);
        TARGET_RESOLUTION_SCANS.set(0);
    }

    pub(super) fn record_function_lookup_scan() {
        FUNCTION_LOOKUP_SCANS.set(FUNCTION_LOOKUP_SCANS.get() + 1);
    }

    pub(super) fn record_target_resolution_scan() {
        TARGET_RESOLUTION_SCANS.set(TARGET_RESOLUTION_SCANS.get() + 1);
    }

    pub(super) fn snapshot() -> (usize, usize) {
        (FUNCTION_LOOKUP_SCANS.get(), TARGET_RESOLUTION_SCANS.get())
    }
}

pub(crate) fn reachable_entry_module_with_cache(
    module: &SurfaceModule,
    entry: &str,
    entry_kind: FunctionKind,
    cache: &ReachabilityCache,
) -> SurfaceModule {
    let reachability_index = cache
        .function_targets
        .get_or_init(|| reachable_function_targets(module));
    let companion_access_targets = companion_function_access_targets(module);
    let reachable = reachable_functions(
        module,
        entry,
        entry_kind,
        reachability_index,
        &companion_access_targets,
        cache,
    );
    module_with_reachable_functions(module, &reachable)
}

fn reachable_function_targets(module: &SurfaceModule) -> ReachabilityIndex {
    let mut function_targets = function_targets(module);
    function_targets.extend(function_alias_targets(module, &function_targets));
    function_targets.extend(codec_with_targets(module));
    ReachabilityIndex::new(module, function_targets)
}

fn function_targets(module: &SurfaceModule) -> Vec<FunctionTarget> {
    module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(function_target)
        .collect()
}

fn function_target(function: &Function) -> Option<FunctionTarget> {
    let name = function.name.clone()?;
    Some(FunctionTarget {
        name: name.clone(),
        module_name: function.module_name.clone(),
        target_name: name,
        target_module_name: function.module_name.clone(),
        visibility: function.visibility,
        shape: function_shape(function),
        bare_importable: true,
        requires_public_import: false,
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

fn codec_with_targets(module: &SurfaceModule) -> Vec<FunctionTarget> {
    module
        .codecs
        .iter()
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
                        let target = module.functions.iter().find(|function| {
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
                        })
                    }),
            )
        })
        .flatten()
        .collect()
}

fn reachable_functions(
    module: &SurfaceModule,
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
                .function_indices(&key)
                .iter()
                .map(|index| {
                    #[cfg(test)]
                    reachability_counters::record_function_lookup_scan();
                    &module.functions[*index]
                })
                .flat_map(|function| {
                    direct_function_callees(
                        function,
                        module,
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
    module: &SurfaceModule,
    reachable: &HashSet<ReachableFunction>,
) -> SurfaceModule {
    SurfaceModule {
        module: module.module.clone(),
        uses: module.uses.clone(),
        aliases: module.aliases.clone(),
        effects: module.effects.clone(),
        handlers: module.handlers.clone(),
        types: module.types.clone(),
        schemas: module.schemas.clone(),
        codecs: module.codecs.clone(),
        functions: module
            .functions
            .iter()
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
            .cloned()
            .collect(),
    }
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
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct FunctionShape {
    fixed_arity: usize,
    variadic: Option<String>,
}

fn function_alias_targets(
    module: &SurfaceModule,
    function_targets: &[FunctionTarget],
) -> Vec<FunctionTarget> {
    module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
        .filter_map(|alias| {
            let name = alias.name.clone()?;
            let target = target_for_alias_path(
                &alias.target,
                &module.uses,
                function_targets,
                alias.module_name.as_deref(),
            )?;
            if companion_alias_targets_imported_private_function(alias, target) {
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
    uses: &[UseDecl],
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
    uses: &'a [UseDecl],
    function_targets: &'a FunctionTargetIndex,
    companion_access_targets: &'a HashMap<String, String>,
    handlers: &'a [veln_ast::HandlerDecl],
}

fn direct_function_callees(
    function: &Function,
    module: &SurfaceModule,
    function_targets: &FunctionTargetIndex,
    companion_access_targets: &HashMap<String, String>,
) -> Vec<ReachableFunction> {
    let mut callees = Vec::new();
    let context = FunctionCalleeContext {
        current_module: function.module_name.as_deref(),
        uses: &module.uses,
        function_targets,
        companion_access_targets,
        handlers: &module.handlers,
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
    uses: &[UseDecl],
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
    uses: &[UseDecl],
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
            collect_handler_provider_callees(
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
    uses: &[UseDecl],
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
    uses: &[UseDecl],
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

fn collect_handler_provider_callees(
    expr: &Expr,
    current_module: Option<&str>,
    uses: &[UseDecl],
    function_targets: &FunctionTargetIndex,
    companion_access_targets: &HashMap<String, String>,
    handlers: &[veln_ast::HandlerDecl],
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
        for provider in &handler.providers {
            for callee in resolve_function_reference(
                &provider.provider,
                current_module,
                uses,
                function_targets,
                companion_access_targets,
            ) {
                push_reachable(callees, callee);
            }
        }
    }
}

fn resolve_function_reference(
    segments: &[String],
    current_module: Option<&str>,
    uses: &[UseDecl],
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
    uses: &'a [UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let module_path = segments.join("::");
    uses.iter().find(|use_decl| {
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

fn companion_function_access_targets(module: &SurfaceModule) -> HashMap<String, String> {
    module
        .functions
        .iter()
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
    uses: &[UseDecl],
) -> bool {
    let Some(current_module) = current_module else {
        return true;
    };
    if target.module_name.as_deref() == Some(current_module) {
        return true;
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
    use std::{env, fs};

    use veln_ast::{FunctionKind, SurfaceModule, UseOrigin, lower_surface_ast};
    use veln_project::{
        ManifestExport, ManifestField, ManifestLib, ManifestTool, ManifestUnsupportedSection,
        Project, ProjectManifest,
    };
    use veln_source::{LineCol, SourceFile, SourcePath, SourceSpan};
    use veln_syntax::parse;

    use super::{
        EmbeddedStandardModuleEntry, EmbeddedStandardPackage, SurfaceParts,
        load_embedded_standard_package_from, load_project_sources, load_surface_module,
        reachability_counters, reachable_entry_module,
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

    #[test]
    fn reachable_resolution_skips_unrelated_annotated_functions() {
        fn resolution_scans(unrelated_count: usize) -> (usize, usize) {
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

    #[test]
    fn standard_package_loading_keeps_initial_analysis_work_constant_for_unrelated_modules() {
        #[derive(Debug, PartialEq, Eq)]
        struct StandardInitializationWork {
            loaded_modules: Vec<String>,
            parsed_lowered_modules: usize,
            prepared_declarations: usize,
        }

        fn load_synthetic_standard(unrelated_count: usize) -> StandardInitializationWork {
            let mut modules = std::collections::BTreeMap::new();
            for (path, text) in [
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
            ] {
                let module_name =
                    format!("std::{}", path.trim_end_matches(".veln").replace('/', "::"));
                modules.insert(
                    module_name,
                    EmbeddedStandardModuleEntry {
                        path,
                        text,
                        module: std::sync::OnceLock::new(),
                    },
                );
            }
            let standard = EmbeddedStandardPackage { modules };
            let project = Project {
                root: ".".into(),
                files: vec![SourceFile::new(
                    "main.veln",
                    "pub fn main() -> Int\n  1\nend\n",
                )],
                manifest: None,
            };
            let mut diagnostics = Vec::new();
            let mut parts = SurfaceParts::new();
            load_project_sources(&project, &mut diagnostics, &mut parts, None);
            load_embedded_standard_package_from(&standard, &mut diagnostics, &mut parts, true);
            assert!(diagnostics.is_empty(), "{diagnostics:#?}");
            let loaded_modules = loaded_standard_modules(&parts.module);
            let parsed_lowered_modules = standard
                .modules
                .values()
                .filter(|entry| entry.module.get().is_some())
                .count();
            let prepared_declarations = standard_declaration_count(&parts.module);

            let reusable = veln_sema::prepare_current_reusable_standard_surface_module_environment(
                &parts.module,
            );
            let (semantic_diagnostics, checked) =
                veln_sema::check_project_surface_module_with_standard_environment(
                    &parts.module,
                    &reusable,
                );
            assert!(semantic_diagnostics.is_empty(), "{semantic_diagnostics:#?}");
            assert!(checked.diagnostics.is_empty(), "{:#?}", checked.diagnostics);

            StandardInitializationWork {
                loaded_modules,
                parsed_lowered_modules,
                prepared_declarations,
            }
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
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../veln-stdlib/veln");
        let project = Project::discover(root, &[]).expect("standard project should load");

        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
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
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../veln-stdlib/veln");
        let project = Project::discover(root, &[]).expect("standard project should load");
        let (module, diagnostics) = load_surface_module(&project);

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
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../veln-stdlib/veln");
        let mut project = Project::discover(root, &[]).expect("standard project should load");
        project
            .manifest
            .as_mut()
            .expect("standard project manifest")
            .tools
            .push(ManifestTool {
                name: "extra".to_string(),
                fields: Vec::new(),
            });

        let (_, diagnostics) = load_surface_module(&project);

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "manifest.reserved_standard_package"
                && diagnostic.message == "package name `std` is reserved by the Veln toolchain"
        }));
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
                    "  call=provide\n",
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
