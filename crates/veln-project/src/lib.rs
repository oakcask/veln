//! Source discovery, module context, and import roots.

mod companion;
mod discovery;
mod lockfile;
mod manifest;
mod project;

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
    ProjectManifest, read_manifest,
};
pub use project::Project;
