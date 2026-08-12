use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use serde_json::{Value, json};
use veln_analysis::{
    CapturedDependencyProject, DoctestMode, checked_project_diagnostics_with_captured_dependencies,
};
use veln_diagnostics::diagnostic_to_json;
use veln_project::{Project, dependency_root};

use crate::workspace::{SelectedRootKind, Selection};

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
    CheckProjectOutcome::Success(structured)
}

struct Target {
    root: PathBuf,
    root_display: String,
    mode: AnalysisMode,
    input: Option<PathBuf>,
    require_manifest: bool,
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
        let Some(kind) = selection.root_kind(&project) else {
            return Err(domain_failure(
                "project_not_selected",
                "project is not selected in this workspace",
                json!({"project": project, "roots": selection.roots()}),
            ));
        };
        return selected_target(base, &project, kind, source);
    }

    let manifest_roots = selection
        .roots()
        .iter()
        .filter(|root| selection.root_kind(root) == Some(SelectedRootKind::Manifest))
        .map(String::as_str)
        .collect::<Vec<_>>();
    match (manifest_roots.as_slice(), source) {
        ([project], None) => selected_target(base, project, SelectedRootKind::Manifest, None),
        ([project], Some(_)) => selected_target(base, project, SelectedRootKind::Manifest, source),
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
    kind: SelectedRootKind,
    source: Option<&str>,
) -> Result<Target, CheckProjectOutcome> {
    if kind == SelectedRootKind::Manifest && source.is_some() {
        return Err(domain_failure(
            "invalid_query",
            "manifest project analysis does not accept a source",
            json!({"project": project}),
        ));
    }
    if kind == SelectedRootKind::Manifest {
        return Ok(Target {
            root: root_path(base, project),
            root_display: project.to_string(),
            mode: AnalysisMode::Project,
            input: None,
            require_manifest: true,
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
        require_manifest: false,
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

fn capture_stable_project(target: &Target) -> Result<CapturedProject, CaptureError> {
    capture_stable_project_with(|| capture_once(target))
}

fn capture_stable_project_with(
    mut capture: impl FnMut() -> io::Result<CapturedProject>,
) -> Result<CapturedProject, CaptureError> {
    let mut first_error = None;
    for _ in 0..SNAPSHOT_ATTEMPTS {
        let first = capture();
        let second = capture();
        match (first, second) {
            (Ok(first), Ok(second)) if first.key == second.key => {
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

struct CapturedProject {
    project: Project,
    dependencies: Vec<CapturedDependencyProject>,
    key: Value,
}

fn capture_once(target: &Target) -> io::Result<CapturedProject> {
    if target.require_manifest {
        validate_manifest_root(&target.root)?;
    }
    let inputs = target.input.iter().cloned().collect::<Vec<_>>();
    let project = Project::discover(target.root.clone(), &inputs)?;
    if target.require_manifest && project.manifest.is_none() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "selected manifest project no longer has a manifest",
        ));
    }
    let dependencies = dependency_snapshots(&project)?;
    let key = snapshot_key(&project, &dependencies);
    Ok(CapturedProject {
        project,
        dependencies,
        key,
    })
}

fn snapshot_key(project: &Project, dependencies: &[CapturedDependencyProject]) -> Value {
    json!({
        "manifest": project.manifest.as_ref().map(|manifest| &manifest.source_bytes),
        "files": project.files.iter().map(|file| {
            json!({"path": file.path().as_str(), "text": file.text()})
        }).collect::<Vec<_>>(),
        "dependencies": dependencies.iter().map(dependency_snapshot_key).collect::<Vec<_>>(),
    })
}

fn dependency_snapshots(project: &Project) -> io::Result<Vec<CapturedDependencyProject>> {
    let Some(manifest) = &project.manifest else {
        return Ok(Vec::new());
    };
    let mut snapshots = Vec::new();
    for dependency in &manifest.dependencies {
        let Some(source) = dependency.direct_local_source() else {
            continue;
        };
        let dependency_root = dependency_root(&project.root, &source.value);
        let Ok(dependency_project) = Project::discover(dependency_root, &[]) else {
            snapshots.push(CapturedDependencyProject {
                package: dependency.package.clone(),
                source: source.value.clone(),
                project: None,
            });
            continue;
        };
        snapshots.push(CapturedDependencyProject {
            package: dependency.package.clone(),
            source: source.value.clone(),
            project: Some(dependency_project),
        });
    }
    snapshots
        .sort_by(|left, right| (&left.package, &left.source).cmp(&(&right.package, &right.source)));
    Ok(snapshots)
}

fn dependency_snapshot_key(snapshot: &CapturedDependencyProject) -> Value {
    let Some(project) = &snapshot.project else {
        return json!({
            "package": snapshot.package,
            "root": snapshot.source,
            "unavailable": true,
        });
    };
    json!({
        "package": snapshot.package,
        "root": snapshot.source,
        "manifest": project.manifest.as_ref().map(|manifest| &manifest.source_bytes),
        "files": project.files.iter().map(|file| {
            json!({"path": file.path().as_str(), "text": file.text()})
        }).collect::<Vec<_>>()
    })
}

fn validate_manifest_root(root: &Path) -> io::Result<()> {
    let mut current = PathBuf::new();
    for component in root.components() {
        current.push(component.as_os_str());
        if matches!(component, Component::RootDir | Component::Prefix(_)) {
            continue;
        }
        let metadata = fs::symlink_metadata(&current)?;
        if metadata.file_type().is_symlink() {
            return Err(io::Error::other(
                "selected project root traverses a symbolic link",
            ));
        }
        if !metadata.file_type().is_dir() {
            return Err(io::Error::other("selected project root is not a directory"));
        }
    }
    let manifest = root.join("veln.toml");
    let metadata = fs::symlink_metadata(manifest)?;
    if metadata.file_type().is_file() {
        Ok(())
    } else {
        Err(io::Error::other("selected project manifest is not a file"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use veln_project::parse_manifest_text;
    use veln_source::SourceFile;

    #[test]
    fn stable_capture_retries_manifest_source_and_path_set_changes_only_three_times() {
        let cases = [
            (
                "manifest",
                vec![
                    captured_project(vec![("main.veln", clean_source())], Some("name = \"a\"\n")),
                    captured_project(vec![("main.veln", clean_source())], Some("name = \"b\"\n")),
                    captured_project(vec![("main.veln", clean_source())], Some("name = \"a\"\n")),
                    captured_project(vec![("main.veln", clean_source())], Some("name = \"b\"\n")),
                    captured_project(vec![("main.veln", clean_source())], Some("name = \"a\"\n")),
                    captured_project(vec![("main.veln", clean_source())], Some("name = \"b\"\n")),
                ],
            ),
            (
                "source",
                vec![
                    captured_project(vec![("main.veln", "fn main() -> Int\n  1\nend\n")], None),
                    captured_project(vec![("main.veln", "fn main() -> Int\n  2\nend\n")], None),
                    captured_project(vec![("main.veln", "fn main() -> Int\n  1\nend\n")], None),
                    captured_project(vec![("main.veln", "fn main() -> Int\n  2\nend\n")], None),
                    captured_project(vec![("main.veln", "fn main() -> Int\n  1\nend\n")], None),
                    captured_project(vec![("main.veln", "fn main() -> Int\n  2\nend\n")], None),
                ],
            ),
            (
                "path set",
                vec![
                    captured_project(vec![("a.veln", clean_source())], None),
                    captured_project(
                        vec![("a.veln", clean_source()), ("b.veln", clean_source())],
                        None,
                    ),
                    captured_project(vec![("a.veln", clean_source())], None),
                    captured_project(
                        vec![("a.veln", clean_source()), ("b.veln", clean_source())],
                        None,
                    ),
                    captured_project(vec![("a.veln", clean_source())], None),
                    captured_project(
                        vec![("a.veln", clean_source()), ("b.veln", clean_source())],
                        None,
                    ),
                ],
            ),
            (
                "dependency",
                vec![
                    captured_project_with_dependencies(
                        vec![("main.veln", clean_source())],
                        Some(dependency_manifest()),
                        vec![captured_dependency(
                            "dep",
                            "../dep",
                            vec![("lib.veln", clean_source())],
                            Some("name = \"dep\"\n"),
                        )],
                    ),
                    captured_project_with_dependencies(
                        vec![("main.veln", clean_source())],
                        Some(dependency_manifest()),
                        vec![captured_dependency(
                            "dep",
                            "../dep",
                            vec![("lib.veln", "fn answer() -> Int\n  2\nend\n")],
                            Some("name = \"dep\"\n"),
                        )],
                    ),
                    captured_project_with_dependencies(
                        vec![("main.veln", clean_source())],
                        Some(dependency_manifest()),
                        vec![captured_dependency(
                            "dep",
                            "../dep",
                            vec![("lib.veln", clean_source())],
                            Some("name = \"dep\"\n"),
                        )],
                    ),
                    captured_project_with_dependencies(
                        vec![("main.veln", clean_source())],
                        Some(dependency_manifest()),
                        vec![captured_dependency(
                            "dep",
                            "../dep",
                            vec![("lib.veln", "fn answer() -> Int\n  2\nend\n")],
                            Some("name = \"dep\"\n"),
                        )],
                    ),
                    captured_project_with_dependencies(
                        vec![("main.veln", clean_source())],
                        Some(dependency_manifest()),
                        vec![captured_dependency(
                            "dep",
                            "../dep",
                            vec![("lib.veln", clean_source())],
                            Some("name = \"dep\"\n"),
                        )],
                    ),
                    captured_project_with_dependencies(
                        vec![("main.veln", clean_source())],
                        Some(dependency_manifest()),
                        vec![captured_dependency(
                            "dep",
                            "../dep",
                            vec![("lib.veln", "fn answer() -> Int\n  2\nend\n")],
                            Some("name = \"dep\"\n"),
                        )],
                    ),
                ],
            ),
        ];

        for (name, captures) in cases {
            let mut captures = captures.into_iter();
            let result = capture_stable_project_with(|| Ok(captures.next().unwrap()));
            assert!(matches!(result, Err(CaptureError::Changed)), "{name}");
            assert!(captures.next().is_none(), "{name}");
        }
    }

    #[test]
    fn captured_direct_local_dependencies_feed_successful_analysis() {
        let captured = captured_project_with_dependencies(
            vec![(
                "main.veln",
                concat!(
                    "use foo from \"github.com/oakcask/foo\"\n\n",
                    "pub fn main() -> Int\n",
                    "  add_one(1)\n",
                    "end\n",
                ),
            )],
            Some("[dependencies.\"github.com/oakcask/foo\"]\npath = \"vendor/foo\"\n"),
            vec![captured_dependency(
                "github.com/oakcask/foo",
                "vendor/foo",
                vec![(
                    "foo.veln",
                    "pub fn add_one(value: Int) -> Int\n  value + 1\nend\n",
                )],
                Some(concat!(
                    "[package]\n",
                    "name = \"github.com/oakcask/foo\"\n\n",
                    "[lib]\n",
                    "exports = [\"foo.veln\"]\n",
                )),
            )],
        );

        let diagnostics = checked_project_diagnostics_with_captured_dependencies(
            captured.project,
            DoctestMode::Exclude,
            captured.dependencies,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }

    fn captured_project(files: Vec<(&str, &str)>, manifest: Option<&str>) -> CapturedProject {
        captured_project_with_dependencies(files, manifest, Vec::new())
    }

    fn captured_project_with_dependencies(
        files: Vec<(&str, &str)>,
        manifest: Option<&str>,
        dependencies: Vec<CapturedDependencyProject>,
    ) -> CapturedProject {
        let project = Project {
            root: PathBuf::from("."),
            files: files
                .into_iter()
                .map(|(path, text)| SourceFile::new(path, text))
                .collect(),
            manifest: manifest.map(|text| parse_manifest_text("veln.toml", text)),
        };
        let key = snapshot_key(&project, &dependencies);
        CapturedProject {
            project,
            dependencies,
            key,
        }
    }

    fn captured_dependency(
        package: &str,
        source: &str,
        files: Vec<(&str, &str)>,
        manifest: Option<&str>,
    ) -> CapturedDependencyProject {
        CapturedDependencyProject {
            package: package.to_string(),
            source: source.to_string(),
            project: Some(Project {
                root: PathBuf::from(source),
                files: files
                    .into_iter()
                    .map(|(path, text)| SourceFile::new(path, text))
                    .collect(),
                manifest: manifest.map(|text| parse_manifest_text("veln.toml", text)),
            }),
        }
    }

    fn dependency_manifest() -> &'static str {
        "[dependencies.dep]\npath = \"../dep\"\n"
    }

    fn clean_source() -> &'static str {
        "fn main() -> Int\n  1\nend\n"
    }
}
