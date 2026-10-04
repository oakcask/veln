use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{LineCol, SourcePath, SourceSpan, TextRange};

const UTF8_COLUMN_INDEX_BLOCK_BYTES: usize = 256;

#[derive(Clone, Debug)]
pub struct SourceFile {
    path: SourcePath,
    text: Arc<str>,
    line_starts: Vec<usize>,
    utf8_continuation_prefix: Vec<usize>,
    generated_origin: Option<Arc<GeneratedSpanOrigin>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedSourceOrigin {
    original_path: Option<SourcePath>,
    boundary_mappings: GeneratedBoundaryMappings,
    generated_text: Option<Arc<str>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum GeneratedBoundaryMappings {
    Explicit(Arc<BTreeMap<usize, LineCol>>),
    CopiedRegions(Arc<[CopiedSourceRegion]>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CopiedSourceRegion {
    generated: TextRange,
    generated_start: LineCol,
    original_start: LineCol,
    original_end: LineCol,
}

impl GeneratedSourceOrigin {
    pub fn new(
        original_path: Option<SourcePath>,
        boundary_mappings: impl IntoIterator<Item = (usize, LineCol)>,
    ) -> Self {
        Self {
            original_path,
            boundary_mappings: GeneratedBoundaryMappings::Explicit(Arc::new(
                boundary_mappings.into_iter().collect(),
            )),
            generated_text: None,
        }
    }

    pub fn original_path(&self) -> Option<&SourcePath> {
        self.original_path.as_ref()
    }

    pub fn explicit_boundary_count(&self) -> usize {
        match &self.boundary_mappings {
            GeneratedBoundaryMappings::Explicit(mappings) => mappings.len(),
            GeneratedBoundaryMappings::CopiedRegions(_) => 0,
        }
    }

    pub fn copied_region_count(&self) -> usize {
        match &self.boundary_mappings {
            GeneratedBoundaryMappings::Explicit(_) => 0,
            GeneratedBoundaryMappings::CopiedRegions(regions) => regions.len(),
        }
    }

    pub(crate) fn mapped_boundary(&self, generated: LineCol) -> Option<LineCol> {
        match &self.boundary_mappings {
            GeneratedBoundaryMappings::Explicit(mappings) => {
                mappings.get(&generated.offset).copied()
            }
            GeneratedBoundaryMappings::CopiedRegions(regions) => {
                if !self
                    .generated_text
                    .as_ref()
                    .is_some_and(|text| text.is_char_boundary(generated.offset))
                {
                    return None;
                }
                let index =
                    regions.partition_point(|region| region.generated.end < generated.offset);
                let region = regions.get(index)?;
                if generated.offset == region.generated.end {
                    return Some(region.original_end);
                }
                if generated.offset < region.generated.start
                    || generated.line != region.generated_start.line
                {
                    return None;
                }
                Some(LineCol {
                    line: region.original_start.line,
                    column: region.original_start.column
                        + generated
                            .column
                            .saturating_sub(region.generated_start.column),
                    offset: region.original_start.offset
                        + generated.offset.saturating_sub(region.generated.start),
                })
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct GeneratedSpanOrigin {
    source_origin: Option<GeneratedSourceOrigin>,
    detached_original_path: Option<SourcePath>,
    original_start: Option<LineCol>,
    original_end: Option<LineCol>,
}

impl GeneratedSpanOrigin {
    pub fn new(
        original_path: Option<SourcePath>,
        original_start: Option<LineCol>,
        original_end: Option<LineCol>,
    ) -> Self {
        Self {
            source_origin: None,
            detached_original_path: original_path,
            original_start,
            original_end,
        }
    }

    fn mapped(source_origin: GeneratedSourceOrigin) -> Self {
        Self {
            source_origin: Some(source_origin),
            detached_original_path: None,
            original_start: None,
            original_end: None,
        }
    }

    pub fn original_path(&self) -> Option<&SourcePath> {
        self.source_origin
            .as_ref()
            .map_or(self.detached_original_path.as_ref(), |origin| {
                origin.original_path()
            })
    }

    pub fn original_start(&self) -> Option<LineCol> {
        self.original_start
    }

    pub fn original_end(&self) -> Option<LineCol> {
        self.original_end
    }

    pub fn source_origin(&self) -> Option<&GeneratedSourceOrigin> {
        self.source_origin.as_ref()
    }
}

impl PartialEq for GeneratedSpanOrigin {
    fn eq(&self, other: &Self) -> bool {
        self.source_origin == other.source_origin
            && self.original_path() == other.original_path()
            && self.original_start == other.original_start
            && self.original_end == other.original_end
    }
}

impl Eq for GeneratedSpanOrigin {}

impl SourceFile {
    pub fn new(path: impl Into<SourcePath>, text: impl Into<String>) -> Self {
        Self::with_generated_origin(path, text, None)
    }

    pub fn generated(
        path: impl Into<SourcePath>,
        text: impl Into<String>,
        origin_path: Option<SourcePath>,
    ) -> Self {
        Self::with_generated_origin(
            path,
            text,
            Some(GeneratedSourceOrigin::new(origin_path, [])),
        )
    }

    pub fn generated_with_mappings(
        path: impl Into<SourcePath>,
        text: impl Into<String>,
        origin_path: SourcePath,
        boundary_mappings: impl IntoIterator<Item = (usize, LineCol)>,
    ) -> Self {
        Self::with_generated_origin(
            path,
            text,
            Some(GeneratedSourceOrigin::new(
                Some(origin_path),
                boundary_mappings,
            )),
        )
    }

    pub fn generated_with_copied_regions(
        path: impl Into<SourcePath>,
        text: impl Into<String>,
        origin_path: SourcePath,
        copied_regions: impl IntoIterator<Item = (TextRange, LineCol, LineCol)>,
    ) -> Self {
        let mut source = Self::with_generated_origin(path, text, None);
        let mut regions = copied_regions
            .into_iter()
            .map(
                |(generated, original_start, original_end)| CopiedSourceRegion {
                    generated_start: source.line_col(generated.start),
                    generated,
                    original_start,
                    original_end,
                },
            )
            .collect::<Vec<_>>();
        regions.sort_by_key(|region| (region.generated.start, region.generated.end));
        source.generated_origin = Some(Arc::new(GeneratedSpanOrigin::mapped(
            GeneratedSourceOrigin {
                original_path: Some(origin_path),
                boundary_mappings: GeneratedBoundaryMappings::CopiedRegions(regions.into()),
                generated_text: Some(Arc::clone(&source.text)),
            },
        )));
        source
    }

    fn with_generated_origin(
        path: impl Into<SourcePath>,
        text: impl Into<String>,
        generated_origin: Option<GeneratedSourceOrigin>,
    ) -> Self {
        let text: Arc<str> = Arc::from(text.into());
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
        if !text.is_empty() && text.len().is_multiple_of(UTF8_COLUMN_INDEX_BLOCK_BYTES) {
            utf8_continuation_prefix.push(continuation_count);
        }
        Self {
            path: path.into(),
            text,
            line_starts,
            utf8_continuation_prefix,
            generated_origin: generated_origin
                .map(|origin| Arc::new(GeneratedSpanOrigin::mapped(origin))),
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
        self.generated_origin
            .as_ref()
            .map(|origin| origin.original_path())
    }

    pub fn generated_origin(&self) -> Option<&GeneratedSourceOrigin> {
        self.generated_origin
            .as_deref()
            .and_then(GeneratedSpanOrigin::source_origin)
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
        let start = self.line_col(range.start);
        let end = self.line_col(range.end);
        SourceSpan {
            file: self.path.clone(),
            start,
            end,
            generated_origin: self.generated_origin.clone(),
        }
    }
}

fn relative_path(root: &Path, path: &Path) -> String {
    let path = path
        .strip_prefix(root)
        .map_or_else(|_| PathBuf::from(path), PathBuf::from);
    path.to_string_lossy().replace('\\', "/")
}
