//! Source discovery, module context, and import roots.

mod discovery;
mod manifest;
mod project;

#[cfg(test)]
mod tests;

pub use discovery::discover_source_paths;
pub use manifest::{ManifestModule, ProjectManifest};
pub use project::Project;
