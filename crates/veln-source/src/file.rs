use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::{LineCol, SourcePath, SourceSpan, TextRange};

#[derive(Clone, Debug)]
pub struct SourceFile {
    path: SourcePath,
    text: String,
    line_starts: Vec<usize>,
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
        for (index, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(index + 1);
            }
        }
        Self {
            path: path.into(),
            text,
            line_starts,
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
        let column = self.text[line_start..]
            .char_indices()
            .take_while(|(index, _)| line_start + *index < offset)
            .count()
            + 1;
        LineCol {
            line: line_index + 1,
            column,
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
