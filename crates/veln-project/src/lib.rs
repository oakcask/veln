//! Source discovery, module context, and import roots.

mod discovery;
mod project;

#[cfg(test)]
mod tests;

pub use discovery::discover_source_paths;
pub use project::Project;
