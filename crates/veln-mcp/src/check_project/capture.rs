use super::*;
#[cfg(test)]
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

#[cfg(test)]
thread_local! {
    static AFTER_FIRST_STABLE_CAPTURE_HOOK: RefCell<Option<Box<dyn FnMut()>>> =
        RefCell::new(None);
    static CAPTURE_FILE_READS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_capture_file_reads() {
    CAPTURE_FILE_READS.set(0);
}

#[cfg(test)]
pub(crate) fn capture_file_reads() -> usize {
    CAPTURE_FILE_READS.get()
}

pub(super) enum CaptureError {
    Changed,
    Io,
}

#[derive(Default)]
pub(crate) struct CaptureCache {
    files: BTreeMap<PathBuf, CachedFile>,
    total_bytes: usize,
}

struct CachedFile {
    identity: FileIdentity,
    bytes: Vec<u8>,
}

const CAPTURE_CACHE_MAX_FILES: usize = 1024;
const CAPTURE_CACHE_MAX_BYTES: usize = 16 * 1024 * 1024;

impl CaptureCache {
    fn insert(&mut self, path: PathBuf, file: CachedFile) {
        if file.bytes.len() > CAPTURE_CACHE_MAX_BYTES {
            if let Some(previous) = self.files.remove(&path) {
                self.total_bytes = self.total_bytes.saturating_sub(previous.bytes.len());
            }
            return;
        }
        if let Some(previous) = self.files.remove(&path) {
            self.total_bytes = self.total_bytes.saturating_sub(previous.bytes.len());
        }
        if self.files.len() >= CAPTURE_CACHE_MAX_FILES
            || self.total_bytes.saturating_add(file.bytes.len()) > CAPTURE_CACHE_MAX_BYTES
        {
            self.files.clear();
            self.total_bytes = 0;
        }
        self.total_bytes += file.bytes.len();
        self.files.insert(path, file);
    }
}

pub(super) fn capture_stable_project(
    target: &Target,
    cache: &mut CaptureCache,
) -> Result<CapturedProject, CaptureError> {
    capture_stable_project_with(|| capture_once(target, cache))
}

pub(crate) fn capture_navigation_source(
    base: &WorkspaceBase,
    selection: &Selection,
    source: &str,
    cache: &mut CaptureCache,
) -> Result<(CapturedProject, String, NavigationScope), ToolOutcome> {
    let source = validate_source_path(base, source)?;
    stable_navigation_capture_or_failure(base, selection, &source, cache)
        .map(|captured| (captured.project, captured.source, captured.scope))
}

pub(super) struct CapturedNavigationSource {
    pub(super) project: CapturedProject,
    pub(super) source: String,
    pub(super) scope: NavigationScope,
    pub(super) key: Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NavigationScope {
    Project { project: String },
    SingleFile { project: String, source: String },
}

impl NavigationScope {
    pub(crate) fn metadata(&self, generation: u64) -> Value {
        match self {
            Self::Project { project } => json!({
                "mode": "project",
                "generation": generation,
                "project": project,
                "project_wide": true
            }),
            Self::SingleFile { project, source } => json!({
                "mode": "single_file",
                "generation": generation,
                "project": project,
                "source": source,
                "project_wide": false
            }),
        }
    }
}

fn stable_navigation_capture_or_failure(
    base: &WorkspaceBase,
    selection: &Selection,
    source: &str,
    cache: &mut CaptureCache,
) -> Result<CapturedNavigationSource, ToolOutcome> {
    capture_stable_navigation_source_with(|| {
        capture_navigation_source_once(base, selection, source, cache)
    })
    .map_err(|_| {
        domain_failure(
            "snapshot_changed",
            "workspace files changed during capture",
            json!({}),
        )
    })
}

pub(super) fn capture_stable_navigation_source_with(
    capture: impl FnMut() -> io::Result<CapturedNavigationSource>,
) -> Result<CapturedNavigationSource, CaptureError> {
    capture_stable_with(capture, |captured| &captured.key)
}

fn capture_navigation_source_once(
    base: &WorkspaceBase,
    selection: &Selection,
    source: &str,
    cache: &mut CaptureCache,
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
        let captured = capture_once(&target, cache)?;
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
                scope: NavigationScope::Project {
                    project: root.to_string(),
                },
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
    let captured = capture_once(&target, cache)?;
    Ok(CapturedNavigationSource {
        key: json!({
            "mode": "single_file",
            "source": source,
            "inspected_project": inspected_project,
            "project": captured.key.clone(),
        }),
        project: captured,
        source: source.to_string(),
        scope: NavigationScope::SingleFile {
            project: ".".to_string(),
            source: source.to_string(),
        },
    })
}

fn source_beneath_root<'a>(source: &'a str, root: &str) -> Option<&'a str> {
    if root == "." {
        return Some(source);
    }
    source.strip_prefix(root)?.strip_prefix('/')
}

fn navigation_domain_as_io(failure: ToolOutcome) -> io::Error {
    match failure {
        ToolOutcome::DomainFailure { message, .. } => io::Error::other(message),
        ToolOutcome::Success(_) => io::Error::other("unexpected navigation success"),
    }
}

pub(super) fn capture_stable_project_with(
    capture: impl FnMut() -> io::Result<CapturedProject>,
) -> Result<CapturedProject, CaptureError> {
    capture_stable_with(capture, |captured| &captured.key)
}

fn capture_stable_with<T>(
    mut capture: impl FnMut() -> io::Result<T>,
    key: impl Fn(&T) -> &Value,
) -> Result<T, CaptureError> {
    let mut first_error = None;
    for _ in 0..SNAPSHOT_ATTEMPTS {
        let first = capture();
        #[cfg(test)]
        run_after_first_stable_capture_hook();
        let second = capture();
        match (first, second) {
            (Ok(first), Ok(second)) if key(&first) == key(&second) => {
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

#[cfg(test)]
pub(crate) struct StableCaptureHookGuard;

#[cfg(test)]
impl Drop for StableCaptureHookGuard {
    fn drop(&mut self) {
        AFTER_FIRST_STABLE_CAPTURE_HOOK.with(|hook| {
            *hook.borrow_mut() = None;
        });
    }
}

#[cfg(test)]
pub(crate) fn set_after_first_stable_capture_hook(
    hook: impl FnMut() + 'static,
) -> StableCaptureHookGuard {
    AFTER_FIRST_STABLE_CAPTURE_HOOK.with(|current| {
        let mut current = current.borrow_mut();
        assert!(current.is_none(), "stable capture test hook is already set");
        *current = Some(Box::new(hook));
    });
    StableCaptureHookGuard
}

#[cfg(test)]
fn run_after_first_stable_capture_hook() {
    AFTER_FIRST_STABLE_CAPTURE_HOOK.with(|hook| {
        if let Some(hook) = &mut *hook.borrow_mut() {
            hook();
        }
    });
}

pub(crate) struct CapturedProject {
    pub(crate) project: Project,
    pub(crate) dependencies: Vec<CapturedDependencyProject>,
    pub(crate) key: Value,
}

fn capture_once(target: &Target, cache: &mut CaptureCache) -> io::Result<CapturedProject> {
    validate_base_identity(target)?;
    validate_selected_root_identity(target)?;
    let (project, boundary_manifests) = if target.require_manifest {
        let captured = capture_manifest_project(target, cache)?;
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
                files: vec![read_source_file(cache, &root, &target.root, input)?],
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
    let dependencies = dependency_snapshots(&project, cache)?;
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

fn capture_manifest_project(
    target: &Target,
    cache: &mut CaptureCache,
) -> io::Result<ManifestCapture> {
    validate_manifest_root(&target.root)?;
    let root = open_checked_root(target)?;
    let manifest_text = read_text_beneath(cache, &root, &target.root, Path::new("veln.toml"))?;
    let manifest = parse_manifest_text("veln.toml", &manifest_text);
    let mut source_paths = Vec::new();
    let mut boundary_manifests = Vec::new();
    collect_veln_files_beneath(
        &root,
        &target.root,
        Path::new("."),
        &mut source_paths,
        &mut boundary_manifests,
        cache,
    )?;
    source_paths.sort();
    source_paths.dedup();
    boundary_manifests.sort_by(|left, right| left.path.cmp(&right.path));
    let files = source_paths
        .iter()
        .map(|path| read_source_file(cache, &root, &target.root, path))
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
    open_directory_at(base.dir(), path)
}

#[cfg(target_os = "linux")]
fn open_directory_at(parent: &File, path: &Path) -> io::Result<File> {
    let fd = openat2(
        parent,
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
    open_directory_at(parent, path)
}

fn read_source_file(
    cache: &mut CaptureCache,
    root: &File,
    root_path: &Path,
    path: &Path,
) -> io::Result<SourceFile> {
    let text = read_text_beneath(cache, root, root_path, path)?;
    Ok(SourceFile::new(display_relative_path(path), text))
}

#[cfg(target_os = "linux")]
fn read_text_beneath(
    cache: &mut CaptureCache,
    root: &File,
    root_path: &Path,
    path: &Path,
) -> io::Result<String> {
    let bytes = read_bytes_beneath(cache, root, root_path, path)?;
    String::from_utf8(bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.utf8_error()))
}

#[cfg(target_os = "linux")]
fn read_bytes_beneath(
    cache: &mut CaptureCache,
    root: &File,
    root_path: &Path,
    path: &Path,
) -> io::Result<Vec<u8>> {
    read_bytes_and_identity_beneath(cache, root, root_path, path).map(|(bytes, _)| bytes)
}

#[cfg(target_os = "linux")]
fn read_bytes_and_identity_beneath(
    cache: &mut CaptureCache,
    root: &File,
    root_path: &Path,
    path: &Path,
) -> io::Result<(Vec<u8>, FileIdentity)> {
    let mut file = open_regular_file_beneath(root, path, "captured path is not a regular file")?;
    let identity = FileIdentity::from_metadata(&file.metadata()?)?;
    let cache_path = root_path.join(path);
    if let Some(cached) = cache.files.get(&cache_path)
        && cached.identity == identity
    {
        return Ok((cached.bytes.clone(), identity));
    }
    let mut bytes = Vec::new();
    #[cfg(test)]
    CAPTURE_FILE_READS.set(CAPTURE_FILE_READS.get() + 1);
    file.read_to_end(&mut bytes)?;
    let after = FileIdentity::from_metadata(&file.metadata()?)?;
    if identity != after {
        return Err(io::Error::other("captured file changed while reading"));
    }
    cache.insert(
        cache_path,
        CachedFile {
            identity,
            bytes: bytes.clone(),
        },
    );
    Ok((bytes, after))
}

#[cfg(target_os = "linux")]
fn open_regular_file_beneath(
    root: &File,
    path: &Path,
    non_regular_message: &'static str,
) -> io::Result<File> {
    let fd = openat2(
        root,
        path,
        OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )?;
    let file = File::from(fd);
    if !file.metadata()?.file_type().is_file() {
        return Err(io::Error::other(non_regular_message));
    }
    Ok(file)
}

#[cfg(not(target_os = "linux"))]
fn read_text_beneath(
    _cache: &mut CaptureCache,
    _root: &File,
    _root_path: &Path,
    _path: &Path,
) -> io::Result<String> {
    Err(no_handle_relative_capture_support())
}

#[cfg(target_os = "linux")]
fn collect_veln_files_beneath(
    dir: &File,
    _root_path: &Path,
    relative_dir: &Path,
    paths: &mut Vec<PathBuf>,
    boundary_manifests: &mut Vec<BoundaryManifest>,
    cache: &mut CaptureCache,
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
            boundary_manifests.push(read_boundary_manifest(cache, &dir, _root_path, &child)?);
            continue;
        }
        collect_veln_files_beneath(&dir, _root_path, &child, paths, boundary_manifests, cache)?;
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
    _cache: &mut CaptureCache,
) -> io::Result<()> {
    Err(no_handle_relative_capture_support())
}

#[cfg(target_os = "linux")]
fn read_boundary_manifest(
    cache: &mut CaptureCache,
    dir: &File,
    root_path: &Path,
    relative_dir: &Path,
) -> io::Result<BoundaryManifest> {
    let relative_path = relative_dir.join("veln.toml");
    let (bytes, identity) = read_bytes_and_identity_beneath(
        cache,
        dir,
        &root_path.join(relative_dir),
        Path::new("veln.toml"),
    )?;
    Ok(BoundaryManifest {
        path: display_relative_path(&relative_path),
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

fn dependency_snapshots(
    project: &Project,
    cache: &mut CaptureCache,
) -> io::Result<Vec<CapturedDependencyProject>> {
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
        let Ok(dependency_project) = discover_cached_project(&dependency_root, cache) else {
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

fn discover_cached_project(root: &Path, cache: &mut CaptureCache) -> io::Result<Project> {
    let paths = veln_project::companion_analysis_inputs(root, &[])?;
    let files = paths
        .iter()
        .map(|path| {
            let text = read_cached_path(cache, path)?;
            let relative = path
                .strip_prefix(root)
                .map_or_else(|_| path.to_path_buf(), PathBuf::from);
            Ok(SourceFile::new(
                relative.to_string_lossy().replace('\\', "/"),
                text,
            ))
        })
        .collect::<io::Result<Vec<_>>>()?;
    let manifest_path = root.join("veln.toml");
    let manifest = match fs::symlink_metadata(&manifest_path) {
        Ok(metadata) if metadata.file_type().is_file() => Some(parse_manifest_text(
            "veln.toml",
            &read_cached_path(cache, &manifest_path)?,
        )),
        Ok(_) => None,
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    Ok(Project {
        root: root.to_path_buf(),
        files,
        manifest,
    })
}

fn read_cached_path(cache: &mut CaptureCache, path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let identity = FileIdentity::from_metadata(&file.metadata()?)?;
    if let Some(cached) = cache.files.get(path)
        && cached.identity == identity
    {
        return String::from_utf8(cached.bytes.clone())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.utf8_error()));
    }
    let mut bytes = Vec::new();
    #[cfg(test)]
    CAPTURE_FILE_READS.set(CAPTURE_FILE_READS.get() + 1);
    file.read_to_end(&mut bytes)?;
    let after = FileIdentity::from_metadata(&file.metadata()?)?;
    if identity != after {
        return Err(io::Error::other("captured file changed while reading"));
    }
    let text = String::from_utf8(bytes.clone())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.utf8_error()))?;
    cache.insert(path.to_path_buf(), CachedFile { identity, bytes });
    Ok(text)
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

pub(super) fn dependency_snapshot_key(snapshot: &CapturedDependencyProject) -> Value {
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
