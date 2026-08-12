use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use serde_json::{Value, json};
use veln_analysis::{DoctestMode, checked_project_diagnostics};
use veln_diagnostics::diagnostic_to_json;
use veln_project::Project;

use crate::workspace::Selection;

const SNAPSHOT_ATTEMPTS: usize = 3;

pub(crate) enum CheckProjectOutcome {
    Success(Value),
    DomainFailure {
        code: &'static str,
        message: &'static str,
        details: Value,
    },
}

pub(crate) fn check_project(
    base: &Path,
    selection: &Selection,
    arguments: &Value,
) -> CheckProjectOutcome {
    let project = arguments.get("project").and_then(Value::as_str);
    let source = arguments.get("source").and_then(Value::as_str);

    let target = match select_target(base, selection, project, source) {
        Ok(target) => target,
        Err(failure) => return failure,
    };
    let captured = match capture_stable_project(&target.root, target.input.as_deref()) {
        Ok(project) => project,
        Err(CaptureError::Changed | CaptureError::Io) => {
            return domain_failure(
                "snapshot_changed",
                "workspace files changed during capture",
                json!({}),
            );
        }
    };

    let diagnostics = checked_project_diagnostics(captured, DoctestMode::Exclude)
        .iter()
        .map(diagnostic_to_serde)
        .collect::<Vec<_>>();
    let structured = json!({
        "schema_version": 1,
        "analysis": target.metadata(selection.generation()),
        "diagnostics": diagnostics,
        "summary": summary(&diagnostics),
    });
    CheckProjectOutcome::Success(structured)
}

struct Target {
    root: PathBuf,
    root_display: String,
    mode: AnalysisMode,
    input: Option<PathBuf>,
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
    base: &Path,
    selection: &Selection,
    project: Option<&str>,
    source: Option<&str>,
) -> Result<Target, CheckProjectOutcome> {
    if let Some(project) = project {
        let project = validate_project_path(project)?;
        if !selection.roots().contains(&project) {
            return Err(domain_failure(
                "project_not_selected",
                "project is not selected in this workspace",
                json!({"project": project, "roots": selection.roots()}),
            ));
        }
        return selected_target(base, &project, source);
    }

    let manifest_roots = selection
        .roots()
        .iter()
        .filter(|root| is_manifest_project(base, root))
        .cloned()
        .collect::<Vec<_>>();
    match (manifest_roots.as_slice(), source) {
        ([project], None) => selected_target(base, project, None),
        ([project], Some(_)) => selected_target(base, project, source),
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
    base: &Path,
    project: &str,
    source: Option<&str>,
) -> Result<Target, CheckProjectOutcome> {
    if is_manifest_project(base, project) {
        if source.is_some() {
            return Err(domain_failure(
                "invalid_query",
                "manifest project analysis does not accept a source",
                json!({"project": project}),
            ));
        }
        return Ok(Target {
            root: root_path(base, project),
            root_display: project.to_string(),
            mode: AnalysisMode::Project,
            input: None,
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
        root: base.to_path_buf(),
        root_display: ".".to_string(),
        mode: AnalysisMode::SingleFile {
            source: source.clone(),
        },
        input: Some(PathBuf::from(source)),
    })
}

fn validate_project_path(project: &str) -> Result<String, CheckProjectOutcome> {
    let normalized = normalize_relative(project)?;
    if normalized.is_empty() {
        return Err(invalid_path("project path is empty"));
    }
    Ok(normalized)
}

fn validate_source_path(base: &Path, source: &str) -> Result<String, CheckProjectOutcome> {
    reject_spelled_source_traversal(base, source)?;
    let normalized = normalize_relative(source)?;
    if normalized == "." || !normalized.ends_with(".veln") {
        return Err(invalid_path("source must be one `.veln` file"));
    }
    reject_link_or_non_file(base, &normalized)?;
    Ok(normalized)
}

fn reject_spelled_source_traversal(base: &Path, source: &str) -> Result<(), CheckProjectOutcome> {
    let mut current = base.to_path_buf();
    for component in Path::new(source).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if current == base {
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

fn normalize_relative(path: &str) -> Result<String, CheckProjectOutcome> {
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

fn reject_link_or_non_file(base: &Path, source: &str) -> Result<(), CheckProjectOutcome> {
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

fn is_manifest_project(base: &Path, root: &str) -> bool {
    fs::symlink_metadata(root_path(base, root).join("veln.toml"))
        .is_ok_and(|metadata| metadata.file_type().is_file())
}

fn root_path(base: &Path, root: &str) -> PathBuf {
    if root == "." {
        base.to_path_buf()
    } else {
        base.join(root)
    }
}

enum CaptureError {
    Changed,
    Io,
}

fn capture_stable_project(root: &Path, input: Option<&Path>) -> Result<Project, CaptureError> {
    let inputs = input.into_iter().map(PathBuf::from).collect::<Vec<_>>();
    let mut first_error = None;
    for _ in 0..SNAPSHOT_ATTEMPTS {
        let first = capture_once(root, &inputs);
        let second = capture_once(root, &inputs);
        match (first, second) {
            (Ok(first), Ok(second)) if snapshot_key(&first) == snapshot_key(&second) => {
                return Ok(first);
            }
            (Ok(_), Ok(_)) => {}
            (Err(error), _) | (_, Err(error)) => first_error = Some(error),
        }
    }
    if first_error.is_some() {
        Err(CaptureError::Io)
    } else {
        Err(CaptureError::Changed)
    }
}

fn capture_once(root: &Path, inputs: &[PathBuf]) -> io::Result<Project> {
    Project::discover(root.to_path_buf(), inputs)
}

fn snapshot_key(project: &Project) -> Value {
    json!({
        "manifest": fs::read(project.root.join("veln.toml")).ok(),
        "files": project.files.iter().map(|file| {
            json!({"path": file.path().as_str(), "text": file.text()})
        }).collect::<Vec<_>>()
    })
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

fn invalid_path(message: &'static str) -> CheckProjectOutcome {
    domain_failure("invalid_path", message, json!({}))
}

fn domain_failure(
    code: &'static str,
    message: &'static str,
    details: Value,
) -> CheckProjectOutcome {
    CheckProjectOutcome::DomainFailure {
        code,
        message,
        details,
    }
}
