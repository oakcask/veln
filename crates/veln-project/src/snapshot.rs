use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::PortableSourcePathError;
use crate::portable::{default_case_fold, validate_source_path};

mod digest;

pub use digest::{PackageSnapshotDigestError, PackageSnapshotSource, package_snapshot_digest};

/// One owned distribution source retained by a captured package snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturedPackageSource {
    path: String,
    bytes: Vec<u8>,
}

impl CapturedPackageSource {
    /// Returns the deterministic package-relative path with `/` separators.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the exact bytes read from the source file.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// An immutable capture of one package's manifest and distribution sources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturedPackageSnapshot {
    manifest_bytes: Vec<u8>,
    sources: Vec<CapturedPackageSource>,
    digest: String,
}

impl CapturedPackageSnapshot {
    /// Returns the exact bytes read from the package-root `veln.toml`.
    pub fn manifest_bytes(&self) -> &[u8] {
        &self.manifest_bytes
    }

    /// Returns the owned sources in package-relative UTF-8 byte order.
    pub fn sources(&self) -> &[CapturedPackageSource] {
        &self.sources
    }

    /// Returns the package snapshot digest computed from this capture.
    pub fn digest(&self) -> &str {
        &self.digest
    }
}

/// A filesystem package cannot be captured as a package snapshot.
#[derive(Debug)]
pub enum PackageSnapshotCaptureError {
    /// The package root does not contain a regular `veln.toml`.
    ManifestNotRegular(PathBuf),
    /// A discovered entry has no exact UTF-8 package-relative representation.
    UnrepresentablePath(PathBuf),
    /// A represented distribution source path is outside the portable domain.
    InvalidSourcePath {
        path: String,
        reason: PortableSourcePathError,
    },
    /// A distribution source does not contain valid UTF-8 source text.
    InvalidSourceText { path: String, valid_up_to: usize },
    /// Two exact source-path spellings collide under Unicode default case folding.
    SourcePathCollision { first: String, second: String },
    /// A discovered distribution source path is not a regular file.
    SourceNotRegular(PathBuf),
    /// A filesystem operation failed for the retained path.
    Filesystem { path: PathBuf, source: io::Error },
    /// The captured inputs cannot be encoded by the digest transcript.
    Digest(PackageSnapshotDigestError),
}

impl fmt::Display for PackageSnapshotCaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ManifestNotRegular(path) => write!(
                formatter,
                "package snapshot manifest is not a regular file: {}",
                path.display()
            ),
            Self::UnrepresentablePath(path) => write!(
                formatter,
                "package snapshot path is not valid UTF-8: {}",
                path.display()
            ),
            Self::InvalidSourcePath { path, reason } => {
                write!(
                    formatter,
                    "invalid package snapshot source path `{path}`: {reason}"
                )
            }
            Self::InvalidSourceText { path, valid_up_to } => write!(
                formatter,
                "package snapshot source `{path}` is not valid UTF-8 at byte {valid_up_to}"
            ),
            Self::SourcePathCollision { first, second } => write!(
                formatter,
                "package snapshot source paths `{first}` and `{second}` collide after Unicode default case folding"
            ),
            Self::SourceNotRegular(path) => write!(
                formatter,
                "package snapshot source is not a regular file: {}",
                path.display()
            ),
            Self::Filesystem { path, source } => {
                write!(formatter, "cannot capture {}: {source}", path.display())
            }
            Self::Digest(source) => write!(formatter, "cannot digest package snapshot: {source}"),
        }
    }
}

impl Error for PackageSnapshotCaptureError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Filesystem { source, .. } => Some(source),
            Self::Digest(source) => Some(source),
            Self::ManifestNotRegular(_)
            | Self::UnrepresentablePath(_)
            | Self::InvalidSourcePath { .. }
            | Self::InvalidSourceText { .. }
            | Self::SourcePathCollision { .. }
            | Self::SourceNotRegular(_) => None,
        }
    }
}

/// Captures a package root's regular manifest and owned distribution sources.
///
/// Symbolic links, `.git`, descendant package trees, and test sources are
/// excluded. Source bytes are retained exactly and sources are ordered by
/// their package-relative `/`-separated paths before the digest is computed.
pub fn capture_package_snapshot(
    root: &Path,
) -> Result<CapturedPackageSnapshot, PackageSnapshotCaptureError> {
    let manifest_path = root.join("veln.toml");
    let manifest_type = symlink_file_type(&manifest_path)?;
    if !manifest_type.is_file() {
        return Err(PackageSnapshotCaptureError::ManifestNotRegular(
            manifest_path,
        ));
    }
    let manifest_bytes = read_capture_file(&manifest_path)?;

    let mut sources = Vec::new();
    collect_package_sources(root, Path::new(""), &mut sources)?;
    captured_package_snapshot(manifest_bytes, sources)
}

/// Captures an immutable package snapshot from already embedded inputs.
///
/// This applies the same ordering, portable-path, UTF-8, collision, and digest
/// contracts as filesystem capture without materializing the inputs.
pub fn capture_embedded_package_snapshot<'a>(
    manifest_bytes: &[u8],
    sources: impl IntoIterator<Item = PackageSnapshotSource<'a>>,
) -> Result<CapturedPackageSnapshot, PackageSnapshotCaptureError> {
    captured_package_snapshot(
        manifest_bytes.to_vec(),
        sources
            .into_iter()
            .filter(|source| is_distribution_source(source.path))
            .map(|source| CapturedPackageSource {
                path: source.path.to_string(),
                bytes: source.bytes.to_vec(),
            })
            .collect(),
    )
}

fn captured_package_snapshot(
    manifest_bytes: Vec<u8>,
    mut sources: Vec<CapturedPackageSource>,
) -> Result<CapturedPackageSnapshot, PackageSnapshotCaptureError> {
    sources.sort_unstable_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    validate_captured_sources(&sources)?;

    let digest_sources = sources
        .iter()
        .map(|source| PackageSnapshotSource::new(&source.path, &source.bytes))
        .collect::<Vec<_>>();
    let digest = package_snapshot_digest(&manifest_bytes, &digest_sources)
        .map_err(PackageSnapshotCaptureError::Digest)?;

    Ok(CapturedPackageSnapshot {
        manifest_bytes,
        sources,
        digest,
    })
}

fn validate_captured_sources(
    sources: &[CapturedPackageSource],
) -> Result<(), PackageSnapshotCaptureError> {
    let mut folded_paths = BTreeMap::new();
    for source in sources {
        validate_source_path(&source.path).map_err(|reason| {
            PackageSnapshotCaptureError::InvalidSourcePath {
                path: source.path.clone(),
                reason,
            }
        })?;
        if let Err(error) = std::str::from_utf8(&source.bytes) {
            return Err(PackageSnapshotCaptureError::InvalidSourceText {
                path: source.path.clone(),
                valid_up_to: error.valid_up_to(),
            });
        }

        let folded = default_case_fold(&source.path);
        if let Some(first) = folded_paths.insert(folded, source.path.clone()) {
            return Err(PackageSnapshotCaptureError::SourcePathCollision {
                first,
                second: source.path.clone(),
            });
        }
    }
    Ok(())
}

fn collect_package_sources(
    root: &Path,
    relative_dir: &Path,
    sources: &mut Vec<CapturedPackageSource>,
) -> Result<(), PackageSnapshotCaptureError> {
    let directory = root.join(relative_dir);
    let entries =
        fs::read_dir(&directory).map_err(|source| filesystem_error(directory.clone(), source))?;
    for entry in entries {
        let entry = entry.map_err(|source| filesystem_error(directory.clone(), source))?;
        let file_name = entry.file_name();
        let relative_path = relative_dir.join(&file_name);
        let path = root.join(&relative_path);
        if file_name == OsStr::new(".git") {
            continue;
        }

        let file_type = entry
            .file_type()
            .map_err(|source| filesystem_error(path.clone(), source))?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if has_regular_manifest(&path)? {
                continue;
            }
            collect_package_sources(root, &relative_path, sources)?;
        } else {
            if is_excluded_test_source_path(&relative_path) {
                continue;
            }
            let relative_utf8 = package_relative_path(&relative_path)?;
            if is_distribution_source(&relative_utf8) {
                if !file_type.is_file() {
                    return Err(PackageSnapshotCaptureError::SourceNotRegular(path));
                }
                sources.push(CapturedPackageSource {
                    path: relative_utf8,
                    bytes: read_capture_file(&path)?,
                });
            }
        }
    }
    Ok(())
}

fn package_relative_path(path: &Path) -> Result<String, PackageSnapshotCaptureError> {
    let mut normalized = String::new();
    for component in path.components() {
        let component = component
            .as_os_str()
            .to_str()
            .ok_or_else(|| PackageSnapshotCaptureError::UnrepresentablePath(path.to_path_buf()))?;
        if !normalized.is_empty() {
            normalized.push('/');
        }
        normalized.push_str(component);
    }
    Ok(normalized)
}

fn is_distribution_source(path: &str) -> bool {
    path.ends_with(".veln") && !path.ends_with(".test.veln") && !path.ends_with("_test.veln")
}

fn is_excluded_test_source_path(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        is_excluded_test_source_bytes(path.as_os_str().as_bytes())
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        let path = path.as_os_str().encode_wide().collect::<Vec<_>>();
        is_excluded_test_source_wide(&path)
    }

    #[cfg(not(any(unix, windows)))]
    {
        path.to_str()
            .is_some_and(|path| path.ends_with(".test.veln") || path.ends_with("_test.veln"))
    }
}

#[cfg(unix)]
fn is_excluded_test_source_bytes(path: &[u8]) -> bool {
    path.ends_with(b".test.veln") || path.ends_with(b"_test.veln")
}

#[cfg(windows)]
fn is_excluded_test_source_wide(path: &[u16]) -> bool {
    const COMPANION_SUFFIX: &[u16] = &[
        b'.' as u16,
        b't' as u16,
        b'e' as u16,
        b's' as u16,
        b't' as u16,
        b'.' as u16,
        b'v' as u16,
        b'e' as u16,
        b'l' as u16,
        b'n' as u16,
    ];
    const INTEGRATION_SUFFIX: &[u16] = &[
        b'_' as u16,
        b't' as u16,
        b'e' as u16,
        b's' as u16,
        b't' as u16,
        b'.' as u16,
        b'v' as u16,
        b'e' as u16,
        b'l' as u16,
        b'n' as u16,
    ];

    path.ends_with(COMPANION_SUFFIX) || path.ends_with(INTEGRATION_SUFFIX)
}

fn has_regular_manifest(directory: &Path) -> Result<bool, PackageSnapshotCaptureError> {
    let manifest = directory.join("veln.toml");
    match fs::symlink_metadata(&manifest) {
        Ok(metadata) => Ok(metadata.file_type().is_file()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(filesystem_error(manifest, source)),
    }
}

fn symlink_file_type(path: &Path) -> Result<fs::FileType, PackageSnapshotCaptureError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.file_type()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Err(
            PackageSnapshotCaptureError::ManifestNotRegular(path.to_path_buf()),
        ),
        Err(source) => Err(filesystem_error(path.to_path_buf(), source)),
    }
}

fn read_capture_file(path: &Path) -> Result<Vec<u8>, PackageSnapshotCaptureError> {
    fs::read(path).map_err(|source| filesystem_error(path.to_path_buf(), source))
}

fn filesystem_error(path: PathBuf, source: io::Error) -> PackageSnapshotCaptureError {
    PackageSnapshotCaptureError::Filesystem { path, source }
}

#[cfg(test)]
#[path = "snapshot/tests/mod.rs"]
mod tests;
