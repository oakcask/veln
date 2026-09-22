use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::{LineCol, SourcePath, SourceSpan, TextRange};

const UTF8_COLUMN_INDEX_BLOCK_BYTES: usize = 256;

#[derive(Clone, Debug)]
pub struct SourceFile {
    path: SourcePath,
    text: String,
    line_starts: Vec<usize>,
    utf8_continuation_prefix: Vec<usize>,
    generated_origin_path: GeneratedOriginPath,
}

impl SourceFile {
    pub fn new(path: impl Into<SourcePath>, text: impl Into<String>) -> Self {
        Self::with_generated_origin(path, text, GeneratedOriginPath::NotGenerated)
    }

    pub fn generated(
        path: impl Into<SourcePath>,
        text: impl Into<String>,
        origin_path: Option<SourcePath>,
    ) -> Self {
        Self::with_generated_origin(path, text, GeneratedOriginPath::Generated(origin_path))
    }

    fn with_generated_origin(
        path: impl Into<SourcePath>,
        text: impl Into<String>,
        generated_origin_path: GeneratedOriginPath,
    ) -> Self {
        let text = text.into();
        let mut line_starts = vec![0];
        let mut utf8_continuation_prefix = vec![0];
        let mut continuation_count = 0;
        for (index, byte) in text.bytes().enumerate() {
            if index > 0 && index % UTF8_COLUMN_INDEX_BLOCK_BYTES == 0 {
                utf8_continuation_prefix.push(continuation_count);
            }
            if byte == b'\n' {
                line_starts.push(index + 1);
            }
            if byte & 0b1100_0000 == 0b1000_0000 {
                continuation_count += 1;
            }
        }
        if !text.is_empty() && text.len() % UTF8_COLUMN_INDEX_BLOCK_BYTES == 0 {
            utf8_continuation_prefix.push(continuation_count);
        }
        Self {
            path: path.into(),
            text,
            line_starts,
            utf8_continuation_prefix,
            generated_origin_path,
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

    pub fn generated_origin_path(&self) -> Option<Option<&SourcePath>> {
        match &self.generated_origin_path {
            GeneratedOriginPath::NotGenerated => None,
            GeneratedOriginPath::Generated(origin_path) => Some(origin_path.as_ref()),
        }
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
        let continuation_bytes_before_line = self.utf8_continuations_before(line_start);
        let continuation_bytes_before_offset = self.utf8_continuations_before(offset);
        let column = offset
            - line_start
            - (continuation_bytes_before_offset - continuation_bytes_before_line)
            + 1;
        LineCol {
            line: line_index + 1,
            column,
            offset,
        }
    }

    fn utf8_continuations_before(&self, offset: usize) -> usize {
        let block = offset / UTF8_COLUMN_INDEX_BLOCK_BYTES;
        let block_start = block * UTF8_COLUMN_INDEX_BLOCK_BYTES;
        self.utf8_continuation_prefix[block]
            + self.text.as_bytes()[block_start..offset]
                .iter()
                .filter(|byte| **byte & 0b1100_0000 == 0b1000_0000)
                .count()
    }

    pub fn span(&self, range: TextRange) -> SourceSpan {
        SourceSpan {
            file: self.path.clone(),
            start: self.line_col(range.start),
            end: self.line_col(range.end),
        }
    }
}

#[derive(Clone, Debug)]
enum GeneratedOriginPath {
    NotGenerated,
    Generated(Option<SourcePath>),
}

fn relative_path(root: &Path, path: &Path) -> String {
    let path = path
        .strip_prefix(root)
        .map_or_else(|_| PathBuf::from(path), PathBuf::from);
    path.to_string_lossy().replace('\\', "/")
}
