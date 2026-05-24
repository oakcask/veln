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
    use std::time::{SystemTime, UNIX_EPOCH};

    fn assert_line_col(actual: LineCol, line: usize, column: usize, offset: usize) {
        assert_eq!(
            actual,
            LineCol {
                line,
                column,
                offset
            }
        );
    }

    #[test]
    fn normalizes_source_paths() {
        let path = SourcePath::new("././src\\main.veln");

        assert_eq!(path.as_str(), "src/main.veln");
    }

    #[test]
    fn maps_offsets_to_one_based_lines_and_columns() {
        let source = SourceFile::new("src/main.veln", "a\nbc\n");

        assert_line_col(source.line_col(0), 1, 1, 0);
        assert_line_col(source.line_col(2), 2, 1, 2);
        assert_line_col(source.line_col(4), 2, 3, 4);
    }

    #[test]
    fn clamps_offsets_to_end_of_file() {
        let source = SourceFile::new("main.veln", "a\n");

        assert_line_col(source.line_col(usize::MAX), 2, 1, 2);
    }

    #[test]
    fn reports_empty_file_start_position() {
        let source = SourceFile::new("empty.veln", "");

        assert!(source.is_empty());
        assert_eq!(source.len(), 0);
        assert_line_col(source.line_col(0), 1, 1, 0);
    }

    #[test]
    fn builds_spans_from_text_ranges() {
        let source = SourceFile::new("main.veln", "alpha\nbeta\ngamma");

        let span = source.span(TextRange::new(3, 8));

        assert_eq!(span.file.as_str(), "main.veln");
        assert_line_col(span.start, 1, 4, 3);
        assert_line_col(span.end, 2, 3, 8);
    }

    #[test]
    fn text_ranges_create_points_and_cover_ranges() {
        let point = TextRange::at(4);

        assert_eq!(point, TextRange::new(4, 4));
        assert_eq!(
            TextRange::new(8, 10).cover(TextRange::new(3, 6)),
            TextRange::new(3, 10)
        );
    }

    #[test]
    fn reads_files_with_project_relative_paths() -> std::io::Result<()> {
        let test_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root =
            std::env::temp_dir().join(format!("veln-source-test-{}-{test_id}", std::process::id()));
        let temp = TempDir { root: root.clone() };
        let src_dir = root.join("src");
        let file_path = src_dir.join("main.veln");
        std::fs::create_dir_all(&src_dir)?;
        std::fs::write(&file_path, "fn main()\nend\n")?;

        let source = SourceFile::read(&root, &file_path)?;

        assert_eq!(source.path().as_str(), "src/main.veln");
        assert_eq!(source.text(), "fn main()\nend\n");

        drop(temp);
        Ok(())
    }

    struct TempDir {
        root: std::path::PathBuf,
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
}
