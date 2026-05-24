//! Source files, spans, line indexes, and project-relative paths.

mod file;
mod path;
mod span;

#[cfg(test)]
mod tests;

pub use file::SourceFile;
pub use path::SourcePath;
pub use span::{LineCol, SourceSpan, TextRange};
