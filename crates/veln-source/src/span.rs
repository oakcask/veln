use crate::{GeneratedSpanOrigin, SourcePath};
use std::sync::Arc;

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

#[derive(Clone, Debug)]
pub struct SourceSpan {
    pub file: SourcePath,
    pub start: LineCol,
    pub end: LineCol,
    pub generated_origin: Option<Arc<GeneratedSpanOrigin>>,
}

impl PartialEq for SourceSpan {
    fn eq(&self, other: &Self) -> bool {
        self.file == other.file
            && self.start == other.start
            && self.end == other.end
            && self.original_path() == other.original_path()
            && self.original_start() == other.original_start()
            && self.original_end() == other.original_end()
    }
}

impl Eq for SourceSpan {}

impl SourceSpan {
    pub fn original_path(&self) -> Option<&SourcePath> {
        self.generated_origin.as_ref()?.original_path()
    }

    pub fn original_start(&self) -> Option<LineCol> {
        let origin = self.generated_origin.as_ref()?;
        origin
            .source_origin()
            .map_or(origin.original_start(), |source| {
                source.mapped_boundary(self.start)
            })
    }

    pub fn original_end(&self) -> Option<LineCol> {
        let origin = self.generated_origin.as_ref()?;
        origin
            .source_origin()
            .map_or(origin.original_end(), |source| {
                source.mapped_boundary(self.end)
            })
    }

    pub fn resolved_origin(&self) -> Option<Self> {
        Some(Self {
            file: self.original_path()?.clone(),
            start: self.original_start()?,
            end: self.original_end()?,
            generated_origin: None,
        })
    }

    pub fn resolved_or_generated(&self) -> Self {
        self.resolved_origin().unwrap_or_else(|| self.clone())
    }
}
