use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

use veln_ast::{FunctionKind, SurfaceModule, UseOrigin, lower_surface_ast};
use veln_project::{
    ManifestExport, ManifestField, ManifestLib, ManifestTool, ManifestUnsupportedSection, Project,
    ProjectManifest, parse_manifest_text,
};
use veln_source::{LineCol, SourceFile, SourcePath, SourceSpan};
use veln_syntax::parse;

use crate::analysis::{DoctestMode, analyze_project};

use super::{
    CapturedDependencyProject, Diagnostic, EmbeddedStandardModuleEntry, EmbeddedStandardPackage,
    ReachabilityCache, SurfaceParts, embedded_standard_counters, is_toolchain_standard_project,
    load_embedded_standard_package_from, load_project_sources, load_surface_module,
    load_surface_modules_with_captured_dependencies, reachability_counters, reachable_entry_module,
    reachable_entry_module_with_cache, reachable_entry_module_with_standard_cache,
    validate_manifest_exports, validate_reserved_standard_package,
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
        .filter_map(|function| Some((function.module_name.as_deref()?, function.name.as_deref()?)))
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

fn detail_string<'a>(diagnostic: &'a veln_diagnostics::Diagnostic, key: &str) -> Option<&'a str> {
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

fn detail_number(diagnostic: &veln_diagnostics::Diagnostic, key: &str) -> Option<i64> {
    let veln_diagnostics::JsonValue::Object(entries) = &diagnostic.details else {
        return None;
    };
    entries.iter().find_map(|(entry_key, value)| {
        if entry_key == key
            && let veln_diagnostics::JsonValue::Number(value) = value
        {
            Some(*value)
        } else {
            None
        }
    })
}

mod callable_reachability;
mod companion_visibility;
mod contracts_and_imports;
mod external_dependencies;
mod handler_recovery;
mod manifest_exports;
mod reachable_inputs;
mod recovery_selection;
mod source_identity;
mod standard_initialization;
mod standard_integration;
