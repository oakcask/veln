//! Source discovery, module context, and import roots.

mod discovery;
mod lockfile;
mod manifest;
mod project;

#[cfg(test)]
mod tests;

pub use discovery::discover_source_paths;
pub use lockfile::{LockfileGitSelector, LockfilePackage, LockfileSource, ProjectLockfile};
pub use manifest::{
    ManifestDependency, ManifestDependencySelector, ManifestDependencySelectorKind, ManifestExport,
    ManifestField, ManifestLib, ManifestPackage, ManifestTool, ManifestUnsupportedSection,
    ProjectManifest,
};
pub use project::Project;
