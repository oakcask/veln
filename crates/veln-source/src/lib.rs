//! Source files, spans, line indexes, and project-relative paths.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourcePath(String);

impl SourcePath {
    pub fn new(path: impl Into<String>) -> Self {
        let mut path = path.into().replace('\\', "/");
        while let Some(stripped) = path.strip_prefix("./") {
            path = stripped.to_string();
        }
        Self(path)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for SourcePath {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for SourcePath {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Debug)]
pub struct SourceFile {
    path: SourcePath,
    text: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(path: impl Into<SourcePath>, text: impl Into<String>) -> Self {
        let text = text.into();
        let mut line_starts = vec![0];
        for (index, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(index + 1);
            }
        }
        Self {
            path: path.into(),
            text,
            line_starts,
        }
    }

    pub fn read(project_root: &Path, path: &Path) -> io::Result<Self> {
        let text = fs::read_to_string(path)?;
        let relative = relative_path(project_root, path);
        Ok(Self::new(relative, text))
    }

    pub fn path(&self) -> &SourcePath {
        &self.path
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn line_col(&self, offset: usize) -> LineCol {
        let offset = offset.min(self.text.len());
        let line_index = match self.line_starts.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        let line_start = self.line_starts[line_index];
        LineCol {
            line: line_index + 1,
            column: offset.saturating_sub(line_start) + 1,
            offset,
        }
    }

    pub fn span(&self, range: TextRange) -> SourceSpan {
        SourceSpan {
            file: self.path.clone(),
            start: self.line_col(range.start),
            end: self.line_col(range.end),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn at(offset: usize) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    pub fn cover(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineCol {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSpan {
    pub file: SourcePath,
    pub start: LineCol,
    pub end: LineCol,
}

fn relative_path(root: &Path, path: &Path) -> String {
    let path = path
        .strip_prefix(root)
        .map_or_else(|_| PathBuf::from(path), PathBuf::from);
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_offsets_to_one_based_lines_and_columns() {
        let source = SourceFile::new("src/main.veln", "a\nbc\n");

        assert_eq!(source.line_col(0).line, 1);
        assert_eq!(source.line_col(0).column, 1);
        assert_eq!(source.line_col(2).line, 2);
        assert_eq!(source.line_col(2).column, 1);
        assert_eq!(source.line_col(4).line, 2);
        assert_eq!(source.line_col(4).column, 3);
    }
}
