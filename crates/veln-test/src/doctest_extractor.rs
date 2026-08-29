use super::*;

pub(super) struct DoctestExtractor<'a> {
    source: &'a SourceFile,
    signatures: &'a BTreeMap<String, Option<String>>,
    extracted: ExtractedDoctests,
    pending: Option<ExtractedDoctest>,
    fence: Option<Fence>,
    offset: usize,
}

impl<'a> DoctestExtractor<'a> {
    pub(super) fn new(
        source: &'a SourceFile,
        signatures: &'a BTreeMap<String, Option<String>>,
    ) -> Self {
        Self {
            source,
            signatures,
            extracted: ExtractedDoctests::default(),
            pending: None,
            fence: None,
            offset: 0,
        }
    }

    pub(super) fn extract(mut self) -> ExtractedDoctests {
        for raw_line in self.source.text().split_inclusive('\n') {
            self.handle_raw_line(raw_line);
        }
        self.finalize_pending();
        self.extracted
    }

    pub(super) fn handle_raw_line(&mut self, raw_line: &str) {
        let line = raw_line
            .strip_suffix('\n')
            .unwrap_or(raw_line)
            .strip_suffix('\r')
            .unwrap_or_else(|| raw_line.strip_suffix('\n').unwrap_or(raw_line));
        let line_range = TextRange::new(self.offset, self.offset + line.len());
        self.offset += raw_line.len();

        let Some(content) = doc_comment_content(line) else {
            self.finalize_pending_with_error_context(line);
            return;
        };
        self.handle_doc_line(content.strip_prefix(' ').unwrap_or(content), line_range);
    }

    pub(super) fn handle_doc_line(&mut self, content: &str, line_range: TextRange) {
        if self.fence.is_some() {
            if content.trim_start().starts_with("```") {
                self.close_fence();
            } else {
                self.append_fence_line(content);
            }
            return;
        }

        let trimmed = content.trim_start();
        if let Some(info) = trimmed.strip_prefix("```") {
            self.open_fence(info.trim(), line_range);
        } else if !trimmed.is_empty() {
            self.finalize_pending();
        }
    }

    pub(super) fn open_fence(&mut self, info: &str, line_range: TextRange) {
        if veln_fence_info(info) {
            let span = self.source.span(line_range);
            self.extracted
                .diagnostics
                .extend(veln_metadata_diagnostics(info, span.clone()));
            self.fence = Some(Fence::Veln {
                lines: Vec::new(),
                visible_lines: Vec::new(),
                error_type: doctest_error_type(info).map(ToString::to_string),
                expected_runtime_failure: doctest_runtime_failure(info, span).map(Box::new),
                ignored: doctest_ignored(info),
                should_fail: doctest_should_fail(info),
                fail_span: doctest_should_fail(info).then(|| self.source.span(line_range)),
            });
        } else if output_fence_info(info) {
            let span = self.source.span(line_range);
            self.extracted
                .diagnostics
                .extend(output_metadata_diagnostics(info, span.clone()));
            self.fence = output_fence_stream(info).map_or(Some(Fence::Ignored), |stream| {
                Some(Fence::Output {
                    stream: stream.to_string(),
                    lines: Vec::new(),
                    span,
                })
            });
        } else {
            self.finalize_pending();
        }
    }

    pub(super) fn close_fence(&mut self) {
        match self.fence.take().expect("active fence should exist") {
            Fence::Veln {
                lines,
                visible_lines,
                error_type,
                expected_runtime_failure,
                ignored,
                should_fail,
                fail_span,
            } => {
                self.finalize_pending();
                if !ignored {
                    self.pending = Some(ExtractedDoctest {
                        code: lines,
                        visible_code: visible_lines,
                        error_type,
                        expected_output: None,
                        expected_runtime_failure: expected_runtime_failure.map(|failure| *failure),
                        should_fail,
                        fail_span,
                    });
                }
            }
            Fence::Output {
                stream,
                lines,
                span,
            } => self.attach_output(stream, lines, span),
            Fence::Ignored => {}
        }
    }

    pub(super) fn append_fence_line(&mut self, content: &str) {
        match self.fence.as_mut().expect("active fence should exist") {
            Fence::Veln {
                lines,
                visible_lines,
                ..
            } => {
                lines.push(doctest_code_line(content));
                if !content.starts_with("> ") {
                    visible_lines.push(content.to_string());
                }
            }
            Fence::Output { lines, .. } => lines.push(content.to_string()),
            Fence::Ignored => {}
        }
    }

    pub(super) fn attach_output(&mut self, stream: String, lines: Vec<String>, span: SourceSpan) {
        let Some(doctest) = &mut self.pending else {
            return;
        };
        let output = lines.join("\n");
        let expected_output = doctest.expected_output.get_or_insert_default();
        match stream.as_str() {
            "stdout" => {
                if let Some(first_span) = &expected_output.stdout_span {
                    self.extracted
                        .diagnostics
                        .push(duplicate_output_diagnostic(&stream, &span, first_span));
                } else {
                    expected_output.stdout = Some(output);
                    expected_output.stdout_span = Some(span);
                }
            }
            "stderr" => {
                if let Some(first_span) = &expected_output.stderr_span {
                    self.extracted
                        .diagnostics
                        .push(duplicate_output_diagnostic(&stream, &span, first_span));
                } else {
                    expected_output.stderr = Some(output);
                    expected_output.stderr_span = Some(span);
                }
            }
            _ => {}
        }
    }

    pub(super) fn finalize_pending_with_error_context(&mut self, line: &str) {
        if let Some(doctest) = self.pending.take() {
            self.extracted
                .doctests
                .push(with_error_type_context(doctest, line, self.signatures));
        }
    }

    pub(super) fn finalize_pending(&mut self) {
        if let Some(doctest) = self.pending.take() {
            self.extracted.doctests.push(doctest);
        }
    }
}

pub(super) const RUNTIME_ATTRIBUTE: &str = "runtime";
pub(super) const RUNTIME_CONTRACT_KIND: &str = "contract";
pub(super) const RUNTIME_ENSURE_KIND: &str = "ensure";
pub(super) const RUNTIME_RESULT_KIND: &str = "result";
pub(super) const RUNTIME_CONTRACT_ATTRIBUTES: &[&str] =
    &["clause", "predicate", "function", "blame"];
pub(super) const RUNTIME_CONTRACT_REQUIRED_ATTRIBUTES: &[&str] = &["clause", "predicate"];
pub(super) const RUNTIME_ENSURE_ATTRIBUTES: &[&str] = &["predicate", "function", "blame"];
pub(super) const RUNTIME_ENSURE_REQUIRED_ATTRIBUTES: &[&str] = &["predicate"];
pub(super) const RUNTIME_RESULT_VALUE_ATTRIBUTE: &str = "value";
pub(super) const RUNTIME_RESULT_ATTRIBUTES: &[&str] = &[RUNTIME_RESULT_VALUE_ATTRIBUTE];
pub(super) const RUNTIME_RESULT_REQUIRED_ATTRIBUTES: &[&str] = &[RUNTIME_RESULT_VALUE_ATTRIBUTE];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RuntimeExpectationKind {
    Contract,
    Ensure,
    Result,
}

impl RuntimeExpectationKind {
    pub(super) fn from_value(value: &str) -> Option<Self> {
        match value {
            RUNTIME_CONTRACT_KIND => Some(Self::Contract),
            RUNTIME_ENSURE_KIND => Some(Self::Ensure),
            RUNTIME_RESULT_KIND => Some(Self::Result),
            _ => None,
        }
    }

    pub(super) fn allows_attribute(self, attribute: &str) -> bool {
        match self {
            Self::Contract => RUNTIME_CONTRACT_ATTRIBUTES.contains(&attribute),
            Self::Ensure => RUNTIME_ENSURE_ATTRIBUTES.contains(&attribute),
            Self::Result => RUNTIME_RESULT_ATTRIBUTES.contains(&attribute),
        }
    }

    pub(super) fn required_attributes(self) -> &'static [&'static str] {
        match self {
            Self::Contract => RUNTIME_CONTRACT_REQUIRED_ATTRIBUTES,
            Self::Ensure => RUNTIME_ENSURE_REQUIRED_ATTRIBUTES,
            Self::Result => RUNTIME_RESULT_REQUIRED_ATTRIBUTES,
        }
    }

    pub(super) fn empty_attribute_message(self, attribute: &str) -> String {
        match self {
            Self::Contract => format!("empty doctest runtime contract {attribute}"),
            Self::Ensure => format!("empty doctest runtime ensure {attribute}"),
            Self::Result => "empty doctest runtime result value".to_string(),
        }
    }

    pub(super) fn missing_attribute_message(self, attribute: &str) -> String {
        match self {
            Self::Contract => format!("missing doctest runtime contract {attribute}"),
            Self::Ensure => format!("missing doctest runtime ensure {attribute}"),
            Self::Result => "missing doctest runtime result value".to_string(),
        }
    }

    pub(super) fn expected_failure(
        self,
        info: &str,
        span: SourceSpan,
    ) -> Option<ExpectedRuntimeFailure> {
        match self {
            Self::Contract => Some(ExpectedRuntimeFailure::Contract(ExpectedContractFailure {
                clause: metadata_value(info, "clause")?.to_string(),
                predicate: metadata_value(info, "predicate")?.to_string(),
                function: metadata_value(info, "function").map(ToString::to_string),
                blame: metadata_value(info, "blame").map(ToString::to_string),
                span,
            })),
            Self::Ensure => Some(ExpectedRuntimeFailure::ContractClause(
                ExpectedContractFailure {
                    clause: RUNTIME_ENSURE_KIND.to_string(),
                    predicate: metadata_value(info, "predicate")?.to_string(),
                    function: metadata_value(info, "function").map(ToString::to_string),
                    blame: metadata_value(info, "blame").map(ToString::to_string),
                    span,
                },
            )),
            Self::Result => Some(ExpectedRuntimeFailure::Result(ExpectedResultFailure {
                value: metadata_value(info, RUNTIME_RESULT_VALUE_ATTRIBUTE)?.to_string(),
                span,
            })),
        }
    }
}
