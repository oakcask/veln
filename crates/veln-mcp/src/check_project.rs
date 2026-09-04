#[cfg(target_os = "linux")]
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Read};
#[cfg(target_os = "linux")]
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};

#[cfg(target_os = "linux")]
use rustix::fs::{FileType, Mode, OFlags, RawDir, ResolveFlags, openat2};
use serde_json::{Value, json};
use veln_analysis::{
    CapturedDependencyProject, DoctestMode, checked_project_diagnostics_with_captured_dependencies,
};
use veln_diagnostics::diagnostic_to_json;
use veln_project::{Project, parse_manifest_text};
use veln_source::SourceFile;

use crate::language_resources::LanguageResources;
use crate::outcome::{ToolOutcome, domain_failure};
use crate::workspace::{
    FileIdentity, SelectedRootIdentity, SelectedRootKind, Selection, WorkspaceBase,
};

mod capture;

pub(crate) use capture::capture_navigation_source;
#[cfg(test)]
pub(crate) use capture::set_after_first_stable_capture_hook;
use capture::{CaptureError, capture_stable_project};
#[cfg(test)]
use capture::{
    CapturedNavigationSource, CapturedProject, capture_stable_navigation_source_with,
    capture_stable_project_with, dependency_snapshot_key,
};

const SNAPSHOT_ATTEMPTS: usize = 3;

pub(crate) fn check_project(
    base: &WorkspaceBase,
    selection: &Selection,
    language_resources: &mut LanguageResources,
    arguments: &Value,
) -> ToolOutcome {
    let project = arguments.get("project").and_then(Value::as_str);
    let source = arguments.get("source").and_then(Value::as_str);

    let target = match select_target(base, selection, project, source) {
        Ok(target) => target,
        Err(failure) => return failure,
    };
    let captured = match capture_stable_project(&target) {
        Ok(project) => project,
        Err(CaptureError::Changed | CaptureError::Io) => {
            return domain_failure(
                "snapshot_changed",
                "workspace files changed during capture",
                json!({}),
            );
        }
    };

    let dependencies = captured.dependencies.clone();
    let diagnostics = checked_project_diagnostics_with_captured_dependencies(
        captured.project,
        DoctestMode::Exclude,
        captured.dependencies,
    )
    .iter()
    .map(diagnostic_to_serde)
    .collect::<Vec<_>>();
    let structured = json!({
        "schema_version": 1,
        "analysis": target.metadata(selection.generation()),
        "diagnostics": diagnostics,
        "summary": summary(&diagnostics),
    });
    if let Err(error) = language_resources.admit_dependencies(&dependencies) {
        return error.into();
    }
    ToolOutcome::Success(structured)
}

pub(crate) struct Target {
    base: WorkspaceBase,
    root: PathBuf,
    root_display: String,
    mode: AnalysisMode,
    input: Option<PathBuf>,
    require_manifest: bool,
    selected_root_identity: Option<SelectedRootIdentity>,
}

enum AnalysisMode {
    Project,
    SingleFile { source: String },
}

impl Target {
    fn metadata(&self, generation: u64) -> Value {
        match &self.mode {
            AnalysisMode::Project => json!({
                "mode": "project",
                "generation": generation,
                "project": self.root_display,
                "project_wide": true
            }),
            AnalysisMode::SingleFile { source } => json!({
                "mode": "single_file",
                "generation": generation,
                "project": self.root_display,
                "source": source,
                "project_wide": false
            }),
        }
    }
}

fn select_target(
    base: &WorkspaceBase,
    selection: &Selection,
    project: Option<&str>,
    source: Option<&str>,
) -> Result<Target, ToolOutcome> {
    if let Some(project) = project {
        let project = validate_project_path(project)?;
        let Some(kind) = selection.root_kind(&project) else {
            return Err(domain_failure(
                "project_not_selected",
                "project is not selected in this workspace",
                json!({"project": project, "roots": selection.roots()}),
            ));
        };
        let identity = selection.root_identity(&project).cloned();
        return selected_target(base, &project, kind, source, identity);
    }

    let manifest_roots = selection
        .roots()
        .iter()
        .filter(|root| selection.root_kind(root) == Some(SelectedRootKind::Manifest))
        .map(String::as_str)
        .collect::<Vec<_>>();
    match (manifest_roots.as_slice(), source) {
        ([project], None) => selected_target(
            base,
            project,
            SelectedRootKind::Manifest,
            None,
            selection.root_identity(project).cloned(),
        ),
        ([project], Some(_)) => selected_target(
            base,
            project,
            SelectedRootKind::Manifest,
            source,
            selection.root_identity(project).cloned(),
        ),
        ([], _) => Err(domain_failure(
            "source_required",
            "anonymous analysis requires project `.` and one source",
            json!({"project": ".", "roots": selection.roots()}),
        )),
        (_, _) => Err(domain_failure(
            "project_ambiguous",
            "multiple selected projects require an explicit project",
            json!({"roots": selection.roots()}),
        )),
    }
}

fn selected_target(
    base: &WorkspaceBase,
    project: &str,
    kind: SelectedRootKind,
    source: Option<&str>,
    selected_root_identity: Option<SelectedRootIdentity>,
) -> Result<Target, ToolOutcome> {
    if kind == SelectedRootKind::Manifest && source.is_some() {
        return Err(domain_failure(
            "invalid_query",
            "manifest project analysis does not accept a source",
            json!({"project": project}),
        ));
    }
    if kind == SelectedRootKind::Manifest {
        return Ok(Target {
            base: base.clone(),
            root: root_path(base.path(), project),
            root_display: project.to_string(),
            mode: AnalysisMode::Project,
            input: None,
            require_manifest: true,
            selected_root_identity,
        });
    }

    if project != "." {
        return Err(domain_failure(
            "project_not_selected",
            "anonymous analysis must use project `.`",
            json!({"project": project}),
        ));
    }
    let Some(source) = source else {
        return Err(domain_failure(
            "source_required",
            "anonymous analysis requires one source",
            json!({"project": "."}),
        ));
    };
    let source = validate_source_path(base, source)?;
    Ok(Target {
        base: base.clone(),
        root: base.path().to_path_buf(),
        root_display: ".".to_string(),
        mode: AnalysisMode::SingleFile {
            source: source.to_string(),
        },
        input: Some(PathBuf::from(source)),
        require_manifest: false,
        selected_root_identity,
    })
}

fn validate_project_path(project: &str) -> Result<String, ToolOutcome> {
    let normalized = normalize_relative(project)?;
    if normalized.is_empty() {
        return Err(invalid_path("project path is empty"));
    }
    Ok(normalized)
}

fn validate_source_path(base: &WorkspaceBase, source: &str) -> Result<String, ToolOutcome> {
    reject_spelled_source_traversal(base, source)?;
    let normalized = normalize_relative(source)?;
    if normalized == "." || !normalized.ends_with(".veln") {
        return Err(invalid_path("source must be one `.veln` file"));
    }
    reject_link_or_non_file(base.path(), &normalized)?;
    Ok(normalized)
}

fn reject_spelled_source_traversal(base: &WorkspaceBase, source: &str) -> Result<(), ToolOutcome> {
    let mut current = base.path().to_path_buf();
    for component in Path::new(source).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if current == base.path() {
                    return Err(invalid_path("path escapes the workspace"));
                }
                current.pop();
            }
            Component::Normal(part) => {
                current.push(part);
                let metadata = fs::symlink_metadata(&current)
                    .map_err(|_| invalid_path("source path does not exist"))?;
                let file_type = metadata.file_type();
                if file_type.is_symlink() {
                    return Err(invalid_path("source path traverses a symbolic link"));
                }
                if !file_type.is_dir() && !file_type.is_file() {
                    return Err(invalid_path("source path is not a regular file"));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(invalid_path("path must be workspace-relative"));
            }
        }
    }
    Ok(())
}

fn normalize_relative(path: &str) -> Result<String, ToolOutcome> {
    if path.is_empty() || path.contains('\\') || Path::new(path).is_absolute() {
        return Err(invalid_path("path must be workspace-relative"));
    }
    let mut parts = Vec::new();
    for component in Path::new(path).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => {
                let Some(part) = part.to_str() else {
                    return Err(invalid_path("path must be UTF-8"));
                };
                parts.push(part.to_string());
            }
            Component::ParentDir => {
                if parts.pop().is_none() {
                    return Err(invalid_path("path escapes the workspace"));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(invalid_path("path must be workspace-relative"));
            }
        }
    }
    Ok(if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    })
}

fn reject_link_or_non_file(base: &Path, source: &str) -> Result<(), ToolOutcome> {
    let mut current = base.to_path_buf();
    let parts = source.split('/').collect::<Vec<_>>();
    for (index, part) in parts.iter().enumerate() {
        current.push(part);
        let metadata = fs::symlink_metadata(&current)
            .map_err(|_| invalid_path("source path does not exist"))?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            return Err(invalid_path("source path traverses a symbolic link"));
        }
        if index + 1 == parts.len() {
            if !file_type.is_file() {
                return Err(invalid_path("source path is not a regular file"));
            }
        } else if !file_type.is_dir() {
            return Err(invalid_path("source parent is not a directory"));
        }
    }
    Ok(())
}

fn root_path(base: &Path, root: &str) -> PathBuf {
    if root == "." {
        base.to_path_buf()
    } else {
        base.join(root)
    }
}

fn diagnostic_to_serde(diagnostic: &veln_diagnostics::Diagnostic) -> Value {
    serde_json::from_str(&diagnostic_to_json(diagnostic).to_json())
        .expect("diagnostic JSON should be valid serde JSON")
}

fn summary(diagnostics: &[Value]) -> Value {
    let mut by_severity = serde_json::Map::new();
    let mut by_kind = serde_json::Map::new();
    for diagnostic in diagnostics {
        if let Some(severity) = diagnostic.get("severity").and_then(Value::as_str) {
            increment(&mut by_severity, severity);
        }
        if let Some(kind) = diagnostic.get("kind").and_then(Value::as_str) {
            increment(&mut by_kind, kind);
        }
    }
    json!({
        "diagnostic_count": diagnostics.len(),
        "by_severity": by_severity,
        "by_kind": by_kind,
    })
}

fn increment(map: &mut serde_json::Map<String, Value>, key: &str) {
    let count = map.get(key).and_then(Value::as_u64).unwrap_or(0) + 1;
    map.insert(key.to_string(), json!(count));
}

fn invalid_path(message: &'static str) -> ToolOutcome {
    domain_failure("invalid_path", message, json!({}))
}

#[cfg(test)]
#[path = "check_project/tests.rs"]
mod tests;
