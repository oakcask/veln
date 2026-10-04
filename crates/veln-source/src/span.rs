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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSpan {
    pub file: SourcePath,
    pub start: LineCol,
    pub end: LineCol,
    pub generated_origin: Option<Arc<GeneratedSpanOrigin>>,
}

impl SourceSpan {
    pub fn resolved_origin(&self) -> Option<Self> {
        let origin = self.generated_origin.as_ref()?;
        Some(Self {
            file: origin.original_path()?.clone(),
            start: origin.original_start()?,
            end: origin.original_end()?,
            generated_origin: None,
        })
    }

    pub fn resolved_or_generated(&self) -> Self {
        self.resolved_origin().unwrap_or_else(|| self.clone())
    }
}
