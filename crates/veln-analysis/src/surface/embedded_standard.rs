use super::*;

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

pub(super) struct EmbeddedStandardPackage {
    pub(super) modules: BTreeMap<String, EmbeddedStandardModuleEntry>,
}

pub(super) struct EmbeddedStandardModuleEntry {
    pub(super) path: String,
    pub(super) lowered: Cow<'static, [u8]>,
    pub(super) module: OnceLock<EmbeddedStandardModule>,
}

pub(super) struct EmbeddedStandardModule {
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
                    rejected_derived_modules: BTreeSet::new(),
                },
                diagnostics: Vec::new(),
            }
        })
    }
}

static EMBEDDED_STANDARD_PACKAGE: OnceLock<EmbeddedStandardPackage> = OnceLock::new();

pub(super) fn load_toolchain_standard_sources(
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
        None,
        None,
    );
}

pub(super) fn load_embedded_standard_package(
    diagnostics: &mut Vec<Diagnostic>,
    parts: &mut SurfaceParts,
    merge_into_parts: bool,
) -> BTreeSet<String> {
    let standard = embedded_standard_package();
    load_embedded_standard_package_from(standard, diagnostics, parts, merge_into_parts)
}

pub(super) fn load_embedded_standard_package_from(
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

pub(super) fn merge_surface_parts(parts: &mut SurfaceParts, additions: &SurfaceParts) {
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
    parts
        .rejected_derived_modules
        .extend(additions.rejected_derived_modules.clone());
}

pub(super) fn add_implicit_standard_prelude_imports(parts: &mut SurfaceParts) {
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

pub(super) fn is_toolchain_standard_project(project: &Project) -> bool {
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
