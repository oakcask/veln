//! Source discovery, module context, and import roots.

mod companion;
mod discovery;
mod lockfile;
mod manifest;
mod portable;
mod project;
mod snapshot;

#[cfg(test)]
mod tests;

pub use companion::{
    CompanionSource, CompanionSourceKind, classify_companion_source, companion_analysis_inputs,
    explicit_companion_inputs, is_companion_source_path, production_analysis_inputs,
};
pub use discovery::discover_source_paths;
pub use lockfile::{
    LockfileGitSelector, LockfilePackage, LockfileSource, LowerHexBytes, ProjectLockfile,
    normalize_lockfile_path, source_tree_checksum, write_lockfile,
};
pub use manifest::{
    ManifestDependency, ManifestDependencySelector, ManifestDependencySelectorKind, ManifestExport,
    ManifestField, ManifestLib, ManifestPackage, ManifestTool, ManifestUnsupportedSection,
    ProjectManifest, parse_manifest_text, read_manifest,
};
pub use portable::{
    PORTABLE_UNICODE_VERSION, PORTABLE_UNICODE_VERSION_STRING, PackageIdentity,
    PackageIdentityError, PortableSourcePathError,
};
pub use project::Project;
pub use snapshot::{
    CapturedPackageSnapshot, CapturedPackageSource, PackageSnapshotCaptureError,
    PackageSnapshotDigestError, PackageSnapshotSource, capture_package_snapshot,
    package_snapshot_digest,
};

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Selects the package root for an analysis started in `start`.
///
/// The returned path is the filesystem identity of `start` or the nearest
/// ancestor with a regular `veln.toml` marker.
pub fn select_package_root(start: &Path) -> io::Result<PathBuf> {
    let start = fs::canonicalize(start)?;
    select_package_root_with(start, |marker| fs::symlink_metadata(marker))
}

fn select_package_root_with(
    start: PathBuf,
    mut marker_metadata: impl FnMut(&Path) -> io::Result<fs::Metadata>,
) -> io::Result<PathBuf> {
    let mut candidate = start.clone();
    loop {
        match marker_metadata(&candidate.join("veln.toml")) {
            Ok(metadata) if metadata.file_type().is_file() => return Ok(candidate),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        if !candidate.pop() {
            return Ok(start);
        }
    }
}
