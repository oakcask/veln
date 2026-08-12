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

use crate::workspace::{
    FileIdentity, SelectedRootIdentity, SelectedRootKind, Selection, WorkspaceBase,
};

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
    base: &WorkspaceBase,
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

fn validate_project_path(project: &str) -> Result<String, CheckProjectOutcome> {
    let normalized = normalize_relative(project)?;
    if normalized.is_empty() {
        return Err(invalid_path("project path is empty"));
    }
    Ok(normalized)
}

fn validate_source_path(base: &WorkspaceBase, source: &str) -> Result<String, CheckProjectOutcome> {
    reject_spelled_source_traversal(base, source)?;
    let normalized = normalize_relative(source)?;
    if normalized == "." || !normalized.ends_with(".veln") {
        return Err(invalid_path("source must be one `.veln` file"));
    }
    reject_link_or_non_file(base.path(), &normalized)?;
    Ok(normalized)
}

fn reject_spelled_source_traversal(
    base: &WorkspaceBase,
    source: &str,
) -> Result<(), CheckProjectOutcome> {
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

pub(crate) struct NavigationCapture {
    pub(crate) project: CapturedProject,
    pub(crate) source: String,
    pub(crate) scope_root: Option<String>,
    pub(crate) project_wide: bool,
}

pub(crate) fn capture_navigation_source(
    base: &WorkspaceBase,
    selection: &Selection,
    source: &str,
) -> Result<NavigationCapture, CheckProjectOutcome> {
    let source = validate_source_path(base, source)?;
    stable_navigation_capture_or_failure(base, selection, &source).map(|captured| {
        NavigationCapture {
            project: captured.project,
            source: captured.source,
            scope_root: captured.scope_root,
            project_wide: captured.project_wide,
        }
    })
}

struct CapturedNavigationSource {
    project: CapturedProject,
    source: String,
    scope_root: Option<String>,
    project_wide: bool,
    key: Value,
}

fn stable_navigation_capture_or_failure(
    base: &WorkspaceBase,
    selection: &Selection,
    source: &str,
) -> Result<CapturedNavigationSource, CheckProjectOutcome> {
    capture_stable_navigation_source_with(|| {
        capture_navigation_source_once(base, selection, source)
    })
    .map_err(|_| {
        domain_failure(
            "snapshot_changed",
            "workspace files changed during capture",
            json!({}),
        )
    })
}

fn capture_stable_navigation_source_with(
    mut capture: impl FnMut() -> io::Result<CapturedNavigationSource>,
) -> Result<CapturedNavigationSource, CaptureError> {
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

fn capture_navigation_source_once(
    base: &WorkspaceBase,
    selection: &Selection,
    source: &str,
) -> io::Result<CapturedNavigationSource> {
    let mut inspected_project = None;
    for root in selection.roots() {
        if selection.root_kind(root) != Some(SelectedRootKind::Manifest) {
            continue;
        }
        let Some(relative) = source_beneath_root(source, root) else {
            continue;
        };
        let target = selected_target(
            base,
            root,
            SelectedRootKind::Manifest,
            None,
            selection.root_identity(root).cloned(),
        )
        .map_err(navigation_domain_as_io)?;
        let captured = capture_once(&target)?;
        if captured
            .project
            .files
            .iter()
            .any(|file| file.path().as_str() == relative)
        {
            return Ok(CapturedNavigationSource {
                key: json!({
                    "mode": "selected_project",
                    "root": root,
                    "source": relative,
                    "project": captured.key.clone(),
                }),
                project: captured,
                source: relative.to_string(),
                scope_root: Some(root.to_string()),
                project_wide: true,
            });
        }
        inspected_project = Some(json!({
            "root": root,
            "project": captured.key.clone(),
        }));
        break;
    }

    let target = Target {
        base: base.clone(),
        root: base.path().to_path_buf(),
        root_display: ".".to_string(),
        mode: AnalysisMode::SingleFile {
            source: source.to_string(),
        },
        input: Some(PathBuf::from(&source)),
        require_manifest: false,
        selected_root_identity: None,
    };
    let captured = capture_once(&target)?;
    Ok(CapturedNavigationSource {
        key: json!({
            "mode": "single_file",
            "source": source,
            "inspected_project": inspected_project,
            "project": captured.key.clone(),
        }),
        project: captured,
        source: source.to_string(),
        scope_root: None,
        project_wide: false,
    })
}

fn source_beneath_root<'a>(source: &'a str, root: &str) -> Option<&'a str> {
    if root == "." {
        return Some(source);
    }
    source.strip_prefix(root)?.strip_prefix('/')
}

fn navigation_domain_as_io(failure: CheckProjectOutcome) -> io::Error {
    match failure {
        CheckProjectOutcome::DomainFailure { message, .. } => io::Error::other(message),
        CheckProjectOutcome::Success(_) => io::Error::other("unexpected navigation success"),
    }
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

pub(crate) struct CapturedProject {
    pub(crate) project: Project,
    dependencies: Vec<CapturedDependencyProject>,
    key: Value,
}

fn capture_once(target: &Target) -> io::Result<CapturedProject> {
    validate_base_identity(target)?;
    validate_selected_root_identity(target)?;
    let (project, boundary_manifests) = if target.require_manifest {
        let captured = capture_manifest_project(target)?;
        (captured.project, captured.boundary_manifests)
    } else {
        let input = target.input.as_ref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "anonymous capture requires one input source",
            )
        })?;
        let root = open_checked_root(target)?;
        (
            Project {
                root: target.root.clone(),
                files: vec![read_source_file(&root, &target.root, input)?],
                manifest: None,
            },
            Vec::new(),
        )
    };
    if target.require_manifest && project.manifest.is_none() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "selected manifest project no longer has a manifest",
        ));
    }
    let dependencies = dependency_snapshots(&project)?;
    let key = snapshot_key(&project, &dependencies, &boundary_manifests)?;
    Ok(CapturedProject {
        project,
        dependencies,
        key,
    })
}

struct ManifestCapture {
    project: Project,
    boundary_manifests: Vec<BoundaryManifest>,
}

struct BoundaryManifest {
    path: String,
    identity: FileIdentity,
    bytes: Vec<u8>,
}

fn capture_manifest_project(target: &Target) -> io::Result<ManifestCapture> {
    validate_manifest_root(&target.root)?;
    let root = open_checked_root(target)?;
    let manifest_text = read_text_beneath(&root, &target.root, Path::new("veln.toml"))?;
    let manifest = parse_manifest_text("veln.toml", &manifest_text);
    let mut source_paths = Vec::new();
    let mut boundary_manifests = Vec::new();
    collect_veln_files_beneath(
        &root,
        &target.root,
        Path::new("."),
        &mut source_paths,
        &mut boundary_manifests,
    )?;
    source_paths.sort();
    source_paths.dedup();
    boundary_manifests.sort_by(|left, right| left.path.cmp(&right.path));
    let files = source_paths
        .iter()
        .map(|path| read_source_file(&root, &target.root, path))
        .collect::<io::Result<Vec<_>>>()?;
    Ok(ManifestCapture {
        project: Project {
            root: target.root.clone(),
            files,
            manifest: Some(manifest),
        },
        boundary_manifests,
    })
}

fn open_checked_root(target: &Target) -> io::Result<File> {
    open_dir_beneath(&target.base, Path::new(&target.root_display))
}

#[cfg(target_os = "linux")]
fn open_dir_beneath(base: &WorkspaceBase, path: &Path) -> io::Result<File> {
    let fd = openat2(
        base.dir(),
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )?;
    Ok(File::from(fd))
}

#[cfg(not(target_os = "linux"))]
fn open_dir_beneath(_base: &WorkspaceBase, _path: &Path) -> io::Result<File> {
    Err(no_handle_relative_capture_support())
}

#[cfg(target_os = "linux")]
fn open_child_dir_beneath(parent: &File, path: &Path) -> io::Result<File> {
    let fd = openat2(
        parent,
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )?;
    Ok(File::from(fd))
}

fn read_source_file(root: &File, root_path: &Path, path: &Path) -> io::Result<SourceFile> {
    let text = read_text_beneath(root, root_path, path)?;
    Ok(SourceFile::new(display_relative_path(path), text))
}

#[cfg(target_os = "linux")]
fn read_text_beneath(root: &File, _root_path: &Path, path: &Path) -> io::Result<String> {
    let fd = openat2(
        root,
        path,
        OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )?;
    let mut file = File::from(fd);
    if !file.metadata()?.file_type().is_file() {
        return Err(io::Error::other("captured path is not a regular file"));
    }
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    Ok(text)
}

#[cfg(not(target_os = "linux"))]
fn read_text_beneath(_root: &File, _root_path: &Path, _path: &Path) -> io::Result<String> {
    Err(no_handle_relative_capture_support())
}

#[cfg(target_os = "linux")]
fn collect_veln_files_beneath(
    dir: &File,
    _root_path: &Path,
    relative_dir: &Path,
    paths: &mut Vec<PathBuf>,
    boundary_manifests: &mut Vec<BoundaryManifest>,
) -> io::Result<()> {
    let mut children = Vec::new();
    let mut buffer = Vec::with_capacity(8192);
    let mut entries = RawDir::new(dir, buffer.spare_capacity_mut());
    while let Some(entry) = entries.next() {
        let entry = entry?;
        let name = os_str_from_cstr(entry.file_name())?;
        if name == OsStr::new(".") || name == OsStr::new("..") || name == OsStr::new(".git") {
            continue;
        }
        let child = relative_dir.join(name);
        match entry.file_type() {
            FileType::Directory => children.push((PathBuf::from(name), child)),
            FileType::RegularFile
                if child
                    .extension()
                    .is_some_and(|extension| extension == "veln") =>
            {
                paths.push(child);
            }
            _ => {}
        }
    }
    children.sort_by(|left, right| left.1.cmp(&right.1));
    for (name, child) in children {
        let dir = open_child_dir_beneath(dir, &name)?;
        if has_regular_file_beneath(&dir, Path::new("veln.toml"))? {
            boundary_manifests.push(read_boundary_manifest(&dir, &child)?);
            continue;
        }
        collect_veln_files_beneath(&dir, _root_path, &child, paths, boundary_manifests)?;
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn collect_veln_files_beneath(
    _dir: &File,
    _root_path: &Path,
    _relative_dir: &Path,
    _paths: &mut Vec<PathBuf>,
    _boundary_manifests: &mut Vec<BoundaryManifest>,
) -> io::Result<()> {
    Err(no_handle_relative_capture_support())
}

#[cfg(target_os = "linux")]
fn read_boundary_manifest(dir: &File, relative_dir: &Path) -> io::Result<BoundaryManifest> {
    let fd = openat2(
        dir,
        Path::new("veln.toml"),
        OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )?;
    let mut file = File::from(fd);
    if !file.metadata()?.file_type().is_file() {
        return Err(io::Error::other(
            "nested manifest boundary is not a regular file",
        ));
    }
    let identity = FileIdentity::from_metadata(&file.metadata()?)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(BoundaryManifest {
        path: display_relative_path(&relative_dir.join("veln.toml")),
        identity,
        bytes,
    })
}

#[cfg(target_os = "linux")]
fn has_regular_file_beneath(root: &File, path: &Path) -> io::Result<bool> {
    match openat2(
        root,
        path,
        OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    ) {
        Ok(fd) => Ok(File::from(fd).metadata()?.file_type().is_file()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) if path == Path::new("veln.toml") && linux_open_rejected_non_regular(&error) => {
            Ok(false)
        }
        Err(error) => Err(error.into()),
    }
}

#[cfg(target_os = "linux")]
fn linux_open_rejected_non_regular(error: &rustix::io::Errno) -> bool {
    matches!(
        *error,
        rustix::io::Errno::LOOP | rustix::io::Errno::NOTDIR | rustix::io::Errno::ISDIR
    )
}

#[cfg(target_os = "linux")]
fn os_str_from_cstr(value: &std::ffi::CStr) -> io::Result<&OsStr> {
    Ok(OsStr::from_bytes(value.to_bytes()))
}

fn display_relative_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::CurDir => None,
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => Some(component.as_os_str().to_string_lossy().into_owned()),
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn snapshot_key(
    project: &Project,
    dependencies: &[CapturedDependencyProject],
    boundary_manifests: &[BoundaryManifest],
) -> io::Result<Value> {
    Ok(json!({
        "root": FileIdentity::read(&project.root)?.to_json(),
        "manifest_identity": match &project.manifest {
            Some(_) => Some(FileIdentity::read(&project.root.join("veln.toml"))?.to_json()),
            None => None,
        },
        "manifest": project.manifest.as_ref().map(|manifest| &manifest.source_bytes),
        "files": project.files.iter().map(|file| {
            let path = project.root.join(file.path().as_str());
            Ok(json!({
                "path": file.path().as_str(),
                "identity": FileIdentity::read(&path)?.to_json(),
                "text": file.text()
            }))
        }).collect::<io::Result<Vec<_>>>()?,
        "boundary_manifests": boundary_manifests.iter().map(|boundary| {
            json!({
                "path": boundary.path,
                "identity": boundary.identity.to_json(),
                "bytes": boundary.bytes,
            })
        }).collect::<Vec<_>>(),
        "dependencies": dependencies.iter().map(dependency_snapshot_key).collect::<Vec<_>>(),
    }))
}

fn dependency_snapshots(project: &Project) -> io::Result<Vec<CapturedDependencyProject>> {
    let Some(manifest) = &project.manifest else {
        return Ok(Vec::new());
    };
    let mut snapshots = Vec::new();
    for dependency in &manifest.dependencies {
        let source = dependency_snapshot_source(dependency);
        let dependency_root = match dependency.direct_analysis_source_root(&project.root) {
            Ok(Some(root)) => root,
            Ok(None) | Err(_) => {
                snapshots.push(CapturedDependencyProject {
                    package: dependency.package.clone(),
                    source,
                    project: None,
                });
                continue;
            }
        };
        let Ok(dependency_project) = Project::discover(dependency_root, &[]) else {
            snapshots.push(CapturedDependencyProject {
                package: dependency.package.clone(),
                source,
                project: None,
            });
            continue;
        };
        snapshots.push(CapturedDependencyProject {
            package: dependency.package.clone(),
            source,
            project: Some(dependency_project),
        });
    }
    snapshots
        .sort_by(|left, right| (&left.package, &left.source).cmp(&(&right.package, &right.source)));
    Ok(snapshots)
}

fn dependency_snapshot_source(dependency: &veln_project::ManifestDependency) -> String {
    if let Some(source) = dependency.direct_local_source() {
        return source.value.clone();
    }
    if let Some(git) = &dependency.git {
        if let Some(subdir) = &dependency.subdir {
            return format!("{}#{}", git.value, subdir.value);
        }
        return git.value.clone();
    }
    "<unresolved>".to_string()
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

fn validate_selected_root_identity(target: &Target) -> io::Result<()> {
    let Some(expected) = &target.selected_root_identity else {
        return Ok(());
    };
    if expected.matches_current(&target.root)? {
        Ok(())
    } else {
        Err(io::Error::other(
            "selected project root filesystem identity changed",
        ))
    }
}

fn validate_base_identity(target: &Target) -> io::Result<()> {
    if target.base.matches_current()? {
        Ok(())
    } else {
        Err(io::Error::other(
            "workspace base filesystem identity changed",
        ))
    }
}

#[cfg(not(target_os = "linux"))]
fn no_handle_relative_capture_support() -> io::Error {
    io::Error::other("handle-relative no-follow project capture is not supported on this platform")
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
                "locally materialized git dependency",
                vec![
                    captured_project_with_dependencies(
                        vec![("main.veln", clean_source())],
                        Some(git_dependency_manifest()),
                        vec![captured_dependency(
                            "dep",
                            "https://example.invalid/dep.git",
                            vec![("lib.veln", clean_source())],
                            Some("name = \"dep\"\n"),
                        )],
                    ),
                    captured_project_with_dependencies(
                        vec![("main.veln", clean_source())],
                        Some(git_dependency_manifest()),
                        vec![captured_dependency(
                            "dep",
                            "https://example.invalid/dep.git",
                            vec![("lib.veln", "fn answer() -> Int\n  2\nend\n")],
                            Some("name = \"dep\"\n"),
                        )],
                    ),
                    captured_project_with_dependencies(
                        vec![("main.veln", clean_source())],
                        Some(git_dependency_manifest()),
                        vec![captured_dependency(
                            "dep",
                            "https://example.invalid/dep.git",
                            vec![("lib.veln", clean_source())],
                            Some("name = \"dep\"\n"),
                        )],
                    ),
                    captured_project_with_dependencies(
                        vec![("main.veln", clean_source())],
                        Some(git_dependency_manifest()),
                        vec![captured_dependency(
                            "dep",
                            "https://example.invalid/dep.git",
                            vec![("lib.veln", "fn answer() -> Int\n  2\nend\n")],
                            Some("name = \"dep\"\n"),
                        )],
                    ),
                    captured_project_with_dependencies(
                        vec![("main.veln", clean_source())],
                        Some(git_dependency_manifest()),
                        vec![captured_dependency(
                            "dep",
                            "https://example.invalid/dep.git",
                            vec![("lib.veln", clean_source())],
                            Some("name = \"dep\"\n"),
                        )],
                    ),
                    captured_project_with_dependencies(
                        vec![("main.veln", clean_source())],
                        Some(git_dependency_manifest()),
                        vec![captured_dependency(
                            "dep",
                            "https://example.invalid/dep.git",
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
    fn navigation_capture_retries_descendant_boundary_changes_as_one_attempt() {
        let mut captures = [
            captured_navigation_source(
                captured_project(vec![("main.veln", clean_source())], Some("")),
                "nested/main.veln",
                navigation_boundary_key("name = \"a\"\n"),
            ),
            captured_navigation_source(
                captured_project(vec![("main.veln", clean_source())], Some("")),
                "nested/main.veln",
                navigation_boundary_key("name = \"b\"\n"),
            ),
            captured_navigation_source(
                captured_project(vec![("main.veln", clean_source())], Some("")),
                "nested/main.veln",
                navigation_boundary_key("name = \"a\"\n"),
            ),
            captured_navigation_source(
                captured_project(vec![("main.veln", clean_source())], Some("")),
                "nested/main.veln",
                navigation_boundary_key("name = \"b\"\n"),
            ),
            captured_navigation_source(
                captured_project(vec![("main.veln", clean_source())], Some("")),
                "nested/main.veln",
                navigation_boundary_key("name = \"a\"\n"),
            ),
            captured_navigation_source(
                captured_project(vec![("main.veln", clean_source())], Some("")),
                "nested/main.veln",
                navigation_boundary_key("name = \"b\"\n"),
            ),
        ]
        .into_iter();

        let result = capture_stable_navigation_source_with(|| Ok(captures.next().unwrap()));

        assert!(matches!(result, Err(CaptureError::Changed)));
        assert!(captures.next().is_none());
    }

    fn navigation_boundary_key(boundary_text: &str) -> Value {
        json!({
            "mode": "single_file",
            "source": "nested/main.veln",
            "inspected_project": {
                "root": ".",
                "project": {
                    "boundary_manifests": [
                        {"path": "nested/veln.toml", "text": boundary_text}
                    ]
                }
            },
            "project": {"files": [{"path": "nested/main.veln", "text": clean_source()}]}
        })
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

    fn captured_navigation_source(
        project: CapturedProject,
        source: &str,
        key: Value,
    ) -> CapturedNavigationSource {
        CapturedNavigationSource {
            project,
            source: source.to_string(),
            scope_root: None,
            project_wide: false,
            key,
        }
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
        let key = synthetic_snapshot_key(&project, &dependencies);
        CapturedProject {
            project,
            dependencies,
            key,
        }
    }

    fn synthetic_snapshot_key(
        project: &Project,
        dependencies: &[CapturedDependencyProject],
    ) -> Value {
        json!({
            "manifest": project.manifest.as_ref().map(|manifest| &manifest.source_bytes),
            "files": project.files.iter().map(|file| {
                json!({"path": file.path().as_str(), "text": file.text()})
            }).collect::<Vec<_>>(),
            "boundary_manifests": [],
            "dependencies": dependencies.iter().map(dependency_snapshot_key).collect::<Vec<_>>(),
        })
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

    fn git_dependency_manifest() -> &'static str {
        "[dependencies.dep]\ngit = \"https://example.invalid/dep.git\"\nrev = \"abc123\"\n"
    }

    fn clean_source() -> &'static str {
        "fn main() -> Int\n  1\nend\n"
    }
}
