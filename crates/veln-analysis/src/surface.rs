use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};
use std::sync::OnceLock;

use veln_ast::{
    PublicAliasKind, SurfaceModule, UseDecl, UseOrigin, Visibility, decode_surface_module,
    lower_surface_ast, lower_surface_ast_with_module_identity,
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

use source_module_path::derive_visible_with_source_kind as derive_visible_source_module_path_with_source_kind;
pub use source_module_path::{
    derive as derive_source_module_path, derive_export as derive_export_source_module_path,
    invalid_case_rejected_visible_module_path, is_source_path_invalid_case_diagnostic,
};

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
        let exported_source_paths = manifest_export_source_paths(project);
        let mut checked_export_source_paths = BTreeSet::new();
        load_project_sources(
            project,
            &mut diagnostics,
            &mut parts,
            None,
            Some(&exported_source_paths),
            Some(&mut checked_export_source_paths),
        );
        diagnostics.extend(validate_manifest_exports_with_checked_source_paths(
            project,
            &checked_export_source_paths,
        ));
    }
    if toolchain_std {
        diagnostics.extend(validate_manifest_exports(project));
    }
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
        &parts.rejected_derived_modules,
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

#[derive(Clone)]
struct SurfaceParts {
    module: SurfaceModule,
    derived_modules: Vec<(String, SourceFile)>,
    rejected_derived_modules: BTreeSet<String>,
}

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
            rejected_derived_modules: BTreeSet::new(),
        }
    }
}

mod companion_validation;
mod embedded_standard;
mod external_dependencies;
mod manifest_validation;
mod source_loading;
mod unresolved_imports;

use companion_validation::*;
use embedded_standard::*;
use external_dependencies::*;
use manifest_validation::*;
use source_loading::*;
use unresolved_imports::*;

pub use embedded_standard::load_embedded_standard_surface_module;
pub(crate) use embedded_standard::load_embedded_standard_surface_module_for_names;
pub use manifest_validation::{validate_manifest_dependencies, validate_manifest_exports};

mod reachability;

pub(crate) use reachability::{ReachabilityCache, reachable_entry_module_with_standard_cache};
#[cfg(test)]
pub(crate) use reachability::{
    reachability_counters, reachable_entry_module, reachable_entry_module_with_cache,
};

#[cfg(test)]
mod tests;
