//! Source files, spans, line indexes, and project-relative paths.

mod file;
mod path;
mod span;

#[cfg(test)]
mod tests;

pub use file::{GeneratedSourceOrigin, GeneratedSpanOrigin, SourceFile};
pub use path::{SourcePath, VirtualSourcePathError};
pub use span::{LineCol, SourceSpan, TextRange};
