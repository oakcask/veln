use super::*;

impl PackageDocDiagnosticSpan {
    pub(super) fn from_span(source_uri: &str, span: &SourceSpan) -> Self {
        Self {
            source_uri: source_uri.to_string(),
            line: span.start.line,
            column: span.start.column,
            offset: span.start.offset,
        }
    }
}

pub(super) fn parse_diagnostic(
    gate: &str,
    diagnostic: ParseDiagnostic,
    identity: &str,
    snapshot_digest: &str,
) -> PackageDocDiagnostic {
    package_doc_diagnostic(
        gate,
        diagnostic.id.to_string(),
        diagnostic.message,
        diagnostic.span.as_ref(),
        identity,
        snapshot_digest,
    )
}

pub(super) fn module_diagnostic(
    gate: &str,
    diagnostic: Diagnostic,
    identity: &str,
    snapshot_digest: &str,
) -> PackageDocDiagnostic {
    package_doc_diagnostic(
        gate,
        diagnostic.id,
        diagnostic.message,
        diagnostic.span.as_ref(),
        identity,
        snapshot_digest,
    )
}

fn package_doc_diagnostic(
    gate: &str,
    code: String,
    message: String,
    span: Option<&SourceSpan>,
    identity: &str,
    snapshot_digest: &str,
) -> PackageDocDiagnostic {
    PackageDocDiagnostic {
        gate: gate.to_string(),
        code,
        message,
        span: span.map(|span| {
            PackageDocDiagnosticSpan::from_span(
                &source_uri(identity, snapshot_digest, span.file.as_str()),
                span,
            )
        }),
    }
}

pub(super) fn is_doctest_gate_diagnostic(diagnostic: &Diagnostic) -> bool {
    diagnostic.id.starts_with("doctest.")
        || diagnostic
            .span
            .as_ref()
            .is_some_and(|span| span.file.as_str().contains("#doctest-"))
}

pub(super) fn reconcile_package_expected_doctest_failures(
    diagnostics: Vec<Diagnostic>,
    expected_failures: &BTreeMap<String, SourceSpan>,
) -> Vec<Diagnostic> {
    reconcile_expected_doctest_failures_with(
        diagnostics,
        expected_failures,
        "negative doctest produced no parse diagnostics",
        |diagnostic| {
            diagnostic.severity == Severity::Error && diagnostic.kind == DiagnosticKind::Parse
        },
    )
}

pub(super) fn generated_doctest_static_gate_source(source: &SourceFile) -> GeneratedDoctestSource {
    let visible_lines = normalized_generated_doctest_lines(source);
    let (declarations, statements) =
        split_generated_doctest_visible_lines(source.path().as_str(), &visible_lines);

    if declarations.is_empty() {
        unchanged_generated_doctest_source(source, visible_lines.len())
    } else {
        wrapped_generated_doctest_source(source, declarations, statements)
    }
}

fn normalized_generated_doctest_lines(source: &SourceFile) -> Vec<IndexedDoctestLine> {
    let mut visible_lines = source
        .text()
        .lines()
        .skip(1)
        .filter_map(generated_doctest_body_line)
        .enumerate()
        .map(|(index, text)| IndexedDoctestLine {
            index,
            text: text.to_string(),
        })
        .collect::<Vec<_>>();
    if visible_lines
        .last()
        .is_some_and(|line| line.text.trim_start() == "end")
    {
        visible_lines.pop();
    }
    if visible_lines
        .last()
        .is_some_and(|line| matches!(line.text.trim_start(), "()" | "Ok(())"))
    {
        visible_lines.pop();
    }
    visible_lines
}

fn unchanged_generated_doctest_source(
    source: &SourceFile,
    visible_line_count: usize,
) -> GeneratedDoctestSource {
    let line_origins = (0..visible_line_count)
        .map(|index| (index + 2, generated_doctest_line_origin(source, index, 3)))
        .collect();
    GeneratedDoctestSource {
        source: source.clone(),
        line_origins,
    }
}

fn wrapped_generated_doctest_source(
    source: &SourceFile,
    declarations: Vec<IndexedDoctestLine>,
    statements: Vec<IndexedDoctestLine>,
) -> GeneratedDoctestSource {
    let mut text = String::new();
    let mut line_origins = BTreeMap::new();
    let mut generated_line = 1;
    for line in declarations {
        text.push_str(&line.text);
        text.push('\n');
        line_origins.insert(
            generated_line,
            generated_doctest_line_origin(source, line.index, 1),
        );
        generated_line += 1;
    }
    text.push_str("test doctest_body() -> () effects [stdio]\n");
    generated_line += 1;
    for line in statements {
        if line.text.is_empty() {
            text.push('\n');
        } else {
            text.push_str("  ");
            text.push_str(&line.text);
            text.push('\n');
        }
        line_origins.insert(
            generated_line,
            generated_doctest_line_origin(
                source,
                line.index,
                if line.text.is_empty() { 1 } else { 3 },
            ),
        );
        generated_line += 1;
    }
    text.push_str("  ()\nend\n");
    GeneratedDoctestSource {
        source: SourceFile::new(source.path().as_str(), text),
        line_origins,
    }
}

fn generated_doctest_line_origin(
    source: &SourceFile,
    visible_line_index: usize,
    generated_content_column: usize,
) -> DoctestSourceLineOrigin {
    DoctestSourceLineOrigin {
        original_span: source.span(generated_doctest_body_line_range(
            source,
            visible_line_index + 2,
        )),
        generated_content_column,
    }
}

#[derive(Clone, Debug)]
pub(super) struct IndexedDoctestLine {
    index: usize,
    text: String,
}

pub(super) fn generated_doctest_body_line(line: &str) -> Option<&str> {
    line.strip_prefix("  ")
}

pub(super) fn generated_doctest_body_line_range(
    source: &SourceFile,
    line_number: usize,
) -> TextRange {
    let mut offset = 0;
    for (index, raw_line) in source.text().split_inclusive('\n').enumerate() {
        let current_line = index + 1;
        let line = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        if current_line == line_number {
            let start = offset + line.find(|ch| ch != ' ').unwrap_or(line.len());
            return TextRange::new(start, offset + line.len());
        }
        offset += raw_line.len();
    }
    TextRange::at(source.text().len())
}

pub(super) fn doctest_source_locations(
    public_sources: &[SourceFile],
) -> BTreeMap<String, Vec<SourceSpan>> {
    let mut next_index = 1;
    let mut locations = BTreeMap::new();
    for source in public_sources {
        for visible_lines in visible_doctest_source_line_spans(source) {
            let path = format!("{}#doctest-{next_index}_test.veln", source.path().as_str());
            next_index += 1;
            locations.insert(path, visible_lines);
        }
    }
    locations
}

pub(super) fn visible_doctest_source_line_spans(source: &SourceFile) -> Vec<Vec<SourceSpan>> {
    let mut doctests = Vec::new();
    let mut active = false;
    let mut ignored = false;
    let mut visible_lines = Vec::new();
    let mut offset = 0;
    for raw_line in source.text().split_inclusive('\n') {
        let line = raw_line
            .strip_suffix('\n')
            .unwrap_or(raw_line)
            .strip_suffix('\r')
            .unwrap_or_else(|| raw_line.strip_suffix('\n').unwrap_or(raw_line));
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let Some(after_hashes) = trimmed.strip_prefix("##") else {
            active = false;
            ignored = false;
            visible_lines.clear();
            offset += raw_line.len();
            continue;
        };
        let after_hashes_offset = offset + indent + "##".len();
        let content_offset = after_hashes_offset + usize::from(after_hashes.starts_with(' '));
        let content = after_hashes.strip_prefix(' ').unwrap_or(after_hashes);
        let trimmed_content = content.trim_start();
        if active {
            if trimmed_content.starts_with("```") {
                active = false;
                if !ignored {
                    doctests.push(std::mem::take(&mut visible_lines));
                }
                ignored = false;
                visible_lines.clear();
            } else if !content.starts_with("> ") {
                visible_lines.push(source.span(TextRange::new(
                    content_offset,
                    content_offset + content.len(),
                )));
            }
        } else if let Some(info) = trimmed_content.strip_prefix("```")
            && veln_doctest_fence_info(info.trim())
        {
            active = true;
            ignored = doctest_fence_ignored(info.trim());
            visible_lines.clear();
        }
        offset += raw_line.len();
    }
    doctests
}

pub(super) fn veln_doctest_fence_info(info: &str) -> bool {
    info.split_whitespace().next() == Some("veln")
}

pub(super) fn doctest_fence_ignored(info: &str) -> bool {
    info.split_whitespace()
        .skip(1)
        .any(|field| field == "ignore")
}

pub(super) fn remap_doctest_diagnostic(
    mut diagnostic: Diagnostic,
    doctest_source_locations: &BTreeMap<String, Vec<SourceSpan>>,
    static_gate_locations: &BTreeMap<String, BTreeMap<usize, DoctestSourceLineOrigin>>,
) -> Diagnostic {
    let Some(span) = diagnostic.span.clone() else {
        return diagnostic;
    };
    let Some(visible_line_locations) = doctest_source_locations.get(span.file.as_str()) else {
        return diagnostic;
    };
    let (generated_line, column_delta) = if let Some(line_origins) =
        static_gate_locations.get(span.file.as_str())
        && let Some(origin) = line_origins.get(&span.start.line)
    {
        (
            origin.original_span.start.line,
            span.start
                .column
                .saturating_sub(origin.generated_content_column),
        )
    } else {
        (span.start.line, span.start.column.saturating_sub(1))
    };
    let Some(visible_index) = generated_line.checked_sub(2) else {
        return diagnostic;
    };
    let Some(original) = visible_line_locations.get(visible_index) else {
        return diagnostic;
    };
    let start = LineCol {
        line: original.start.line,
        column: original.start.column + column_delta,
        offset: original.start.offset + column_delta,
    };
    diagnostic.span = Some(SourceSpan {
        file: original.file.clone(),
        start,
        end: start,
    });
    diagnostic
}

pub(super) fn split_generated_doctest_visible_lines(
    path: &str,
    visible_lines: &[IndexedDoctestLine],
) -> (Vec<IndexedDoctestLine>, Vec<IndexedDoctestLine>) {
    let mut text = String::new();
    let mut line_ranges = Vec::new();
    for line in visible_lines {
        let start = text.len();
        text.push_str(&line.text);
        let end = text.len();
        text.push('\n');
        line_ranges.push(TextRange::new(start, end));
    }
    let parsed = parse(&SourceFile::new(path, text));
    let declaration_spans = parsed
        .tree
        .items
        .iter()
        .map(syntax_item_span)
        .collect::<Vec<_>>();

    let mut declarations = Vec::new();
    let mut statements = Vec::new();
    for (line, range) in visible_lines.iter().zip(line_ranges) {
        if declaration_spans
            .iter()
            .any(|span| ranges_intersect(span, &range))
        {
            declarations.push(line.clone());
        } else {
            statements.push(line.clone());
        }
    }
    (declarations, statements)
}

pub(super) fn syntax_item_span(item: &SyntaxItem) -> TextRange {
    let span = match item {
        SyntaxItem::PublicAlias(alias) => &alias.span,
        SyntaxItem::Effect(effect) => &effect.span,
        SyntaxItem::Handler(handler) => &handler.span,
        SyntaxItem::Type(type_decl) => &type_decl.span,
        SyntaxItem::Schema(schema) => &schema.span,
        SyntaxItem::Function(function) => &function.span,
    };
    TextRange::new(span.start.offset, span.end.offset)
}

pub(super) fn ranges_intersect(left: &TextRange, right: &TextRange) -> bool {
    left.start < right.end && right.start < left.end
}

pub(super) fn public_doctest_source(source: &ParsedPackageSource) -> Option<SourceFile> {
    let public_doc_lines = public_doctest_gate_lines(&source.source, &source.tree);
    if public_doc_lines.is_empty() {
        return None;
    }
    let mut text = String::new();
    for (line_index, raw_line) in source.source.text().split_inclusive('\n').enumerate() {
        if public_doc_lines.contains(&(line_index + 1)) {
            text.push_str(raw_line);
        } else {
            push_offset_preserving_blank(raw_line, &mut text);
        }
    }
    Some(SourceFile::new(source.source.path().as_str(), text))
}

pub(super) fn public_doctest_gate_lines(
    source: &SourceFile,
    tree: &veln_syntax::SyntaxTree,
) -> BTreeSet<usize> {
    let original_lines = source.text().lines().collect::<Vec<_>>();
    let mut included = BTreeSet::new();
    for target_line in public_documentation_lines(tree) {
        let mut block_lines = Vec::new();
        collect_doc_block_before(&original_lines, target_line, &mut block_lines);
        if documentation_lines_are_adr_lite(block_lines.iter().map(|(_, line)| *line)) {
            continue;
        }
        for (line_number, line) in block_lines {
            let content = line
                .trim_start()
                .strip_prefix("##")
                .map(str::trim_start)
                .unwrap_or(line);
            if !content.starts_with("> ") {
                included.insert(line_number);
            }
        }
    }
    included
}

pub(super) fn collect_doc_block_before<'a>(
    lines: &[&'a str],
    target_line: usize,
    output: &mut Vec<(usize, &'a str)>,
) {
    if target_line <= 1 {
        return;
    }
    let mut index = target_line - 2;
    let mut docs = Vec::new();
    while let Some(line) = lines.get(index) {
        if line.trim_start().strip_prefix("##").is_some() {
            docs.push((index + 1, *line));
        } else {
            break;
        }
        if index == 0 {
            break;
        }
        index -= 1;
    }
    docs.reverse();
    output.extend(docs);
}

pub(super) fn push_offset_preserving_blank(raw_line: &str, output: &mut String) {
    for byte in raw_line.bytes() {
        match byte {
            b'\r' => output.push('\r'),
            b'\n' => output.push('\n'),
            _ => output.push(' '),
        }
    }
}
