use super::*;

impl<'a> ManifestParser<'a> {
    pub(super) fn new(path: &'a Path) -> Self {
        Self {
            path,
            command: None,
            cwd: None,
            stdin: None,
            stdin_jsonrpc_file: None,
            stdin_jsonrpc_workspace_file_uri_directives: Vec::new(),
            exit: None,
            repeat: 1,
            env: Vec::new(),
            source_errors: SourceErrorExpectation::Forbidden,
            stdout: StreamExpectation::default(),
            stderr: StreamExpectation::default(),
            help: None,
            json_assertions: Vec::new(),
            result_value_assertions: Vec::new(),
            lsp_assertions: Vec::new(),
            mcp_assertions: Vec::new(),
            file_assertions: Vec::new(),
            diagnostics: Vec::new(),
            manifest_error: None,
            binary_fixtures: Vec::new(),
            output_chunk_lists: Vec::new(),
            tools: ToolSetup::default(),
            requires: Requirements::default(),
            skip: SkipRules::default(),
            section: Section::Root,
            seen_assignments: BTreeSet::new(),
            stdin_operand_count: 0,
            case_text_cache: CaseTextCache::default(),
        }
    }

    pub(super) fn parse_section_header(&mut self, line: &str, line_number: usize) {
        self.section = match line {
            "[stdout]" => Section::Stdout,
            "[stderr]" => Section::Stderr,
            "[help]" => self.parse_help_header(line_number),
            "[requires]" => Section::Requires,
            "[skip]" => Section::Skip,
            "[env]" => Section::Env,
            "[tools]" => Section::Tools,
            "[[json_assert]]" => self.parse_json_assert_header(),
            "[[result_value_assert]]" => self.parse_result_value_assert_header(),
            "[[lsp_assert]]" => self.parse_lsp_assert_header(),
            "[[mcp_assert]]" => self.parse_mcp_assert_header(),
            "[[file_assert]]" => self.parse_file_assert_header(),
            "[[diagnostics]]" => self.parse_diagnostic_header(),
            "[diagnostics.span]" => self.parse_diagnostic_span_header(line_number),
            "[manifest_error]" => self.parse_manifest_error_header(line_number),
            "[[binary_fixture]]" => self.parse_binary_fixture_header(),
            "[[output_chunk_list]]" => self.parse_output_chunk_list_header(),
            _ => manifest_error(self.path, line_number, format!("unknown section `{line}`")),
        };
    }

    pub(super) fn parse_help_header(&mut self, line_number: usize) -> Section {
        if self.help.is_some() {
            manifest_error(self.path, line_number, "duplicate help section");
        }
        self.help = Some(HelpExpectation::default());
        Section::Help
    }

    pub(super) fn parse_json_assert_header(&mut self) -> Section {
        self.json_assertions.push(JsonAssertion {
            path: String::new(),
            operation: None,
        });
        Section::JsonAssert(self.json_assertions.len() - 1)
    }

    pub(super) fn parse_file_assert_header(&mut self) -> Section {
        self.file_assertions.push(FileAssertion {
            path: String::new(),
            equals: None,
            missing: false,
            operation_count: 0,
        });
        Section::FileAssert(self.file_assertions.len() - 1)
    }

    pub(super) fn parse_result_value_assert_header(&mut self) -> Section {
        self.result_value_assertions.push(ResultValueAssertion {
            value_path: String::new(),
            path: String::new(),
            operation: None,
        });
        Section::ResultValueAssert(self.result_value_assertions.len() - 1)
    }

    pub(super) fn parse_lsp_assert_header(&mut self) -> Section {
        self.lsp_assertions.push(LspAssertion {
            id: None,
            method: None,
            occurrence: None,
            path: String::new(),
            path_present: false,
            pointer_tokens: Vec::new(),
            operation: None,
            operation_count: 0,
        });
        Section::LspAssert(self.lsp_assertions.len() - 1)
    }

    pub(super) fn parse_mcp_assert_header(&mut self) -> Section {
        self.mcp_assertions.push(McpAssertion {
            id: None,
            path: String::new(),
            path_present: false,
            pointer_tokens: Vec::new(),
            operation: None,
            operation_count: 0,
        });
        Section::McpAssert(self.mcp_assertions.len() - 1)
    }

    pub(super) fn parse_diagnostic_header(&mut self) -> Section {
        self.diagnostics.push(DiagnosticExpectation {
            id: String::new(),
            severity: None,
            kind: None,
            message: None,
            span: None,
        });
        Section::Diagnostic(self.diagnostics.len() - 1)
    }

    pub(super) fn parse_diagnostic_span_header(&mut self, line_number: usize) -> Section {
        let Some(index) = self.diagnostics.len().checked_sub(1) else {
            manifest_error(
                self.path,
                line_number,
                "diagnostics.span needs a diagnostic",
            );
        };
        if self.diagnostics[index].span.is_none() {
            self.diagnostics[index].span = Some(SpanExpectation::default());
        }
        Section::DiagnosticSpan(index)
    }

    pub(super) fn parse_manifest_error_header(&mut self, line_number: usize) -> Section {
        if self.manifest_error.is_some() {
            manifest_error(self.path, line_number, "duplicate manifest_error section");
        }
        self.manifest_error = Some(ManifestErrorExpectation::default());
        Section::ManifestError
    }

    pub(super) fn parse_binary_fixture_header(&mut self) -> Section {
        self.binary_fixtures.push(BinaryFixtureExpectation {
            name: String::new(),
            schema: None,
            bytes: None,
            consumed: None,
            error: None,
            byte_diagnostic: None,
        });
        Section::BinaryFixture(self.binary_fixtures.len() - 1)
    }

    pub(super) fn parse_output_chunk_list_header(&mut self) -> Section {
        self.output_chunk_lists.push(OutputChunkListExpectation {
            name: String::new(),
            chunks: None,
        });
        Section::OutputChunkList(self.output_chunk_lists.len() - 1)
    }

    pub(super) fn parse_section_key(
        &mut self,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        if !is_accumulating_manifest_key(self.section, key) {
            self.reject_duplicate_assignment(line_number, key);
        }
        match self.section {
            Section::Root => self.parse_root_key(line_number, key, value),
            Section::Stdout => parse_stream_key(
                self.path,
                line_number,
                &mut self.stdout,
                key,
                value,
                true,
                &mut self.case_text_cache,
            ),
            Section::Stderr => parse_stream_key(
                self.path,
                line_number,
                &mut self.stderr,
                key,
                value,
                false,
                &mut self.case_text_cache,
            ),
            Section::Help => parse_help_key(
                self.path,
                line_number,
                self.help.as_mut().expect("help section should exist"),
                key,
                value,
                &mut self.case_text_cache,
            ),
            Section::Requires => self.parse_requires_key(line_number, key, value),
            Section::Skip => self.parse_skip_key(line_number, key, value),
            Section::Env => self
                .env
                .push((key.to_string(), parse_string(self.path, value))),
            Section::Tools => self.parse_tools_key(line_number, key, value),
            Section::JsonAssert(index) => {
                self.parse_json_assert_key(index, line_number, key, value)
            }
            Section::ResultValueAssert(index) => {
                self.parse_result_value_assert_key(index, line_number, key, value)
            }
            Section::LspAssert(index) => self.parse_lsp_assert_key(index, line_number, key, value),
            Section::McpAssert(index) => self.parse_mcp_assert_key(index, line_number, key, value),
            Section::FileAssert(index) => {
                self.parse_file_assert_key(index, line_number, key, value)
            }
            Section::Diagnostic(index) => self.parse_diagnostic_key(index, line_number, key, value),
            Section::DiagnosticSpan(index) => {
                self.parse_diagnostic_span_key(index, line_number, key, value);
            }
            Section::ManifestError => self.parse_manifest_error_key(line_number, key, value),
            Section::BinaryFixture(index) => {
                self.parse_binary_fixture_key(index, line_number, key, value)
            }
            Section::OutputChunkList(index) => {
                self.parse_output_chunk_list_key(index, line_number, key, value)
            }
        }
    }

    pub(super) fn reject_duplicate_assignment(&mut self, line_number: usize, key: &str) {
        let assignment = format!("{:?}:{key}", self.section);
        if !self.seen_assignments.insert(assignment) {
            manifest_error(self.path, line_number, format!("duplicate key `{key}`"));
        }
    }

    pub(super) fn parse_root_key(
        &mut self,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        match key {
            "command" => self.command = Some(parse_string_array(self.path, value)),
            "cwd" => self.cwd = Some(PathBuf::from(parse_string(self.path, value))),
            "stdin" => {
                self.stdin_operand_count += 1;
                self.stdin = Some(parse_string(self.path, value));
            }
            "stdin_file" => {
                self.stdin_operand_count += 1;
                self.stdin = Some(self.case_text_cache.read(self.path, value));
            }
            "stdin_jsonrpc_file" => {
                self.stdin_operand_count += 1;
                let relative = parse_string(self.path, value);
                let mut directives = Vec::new();
                self.stdin = Some(load_jsonrpc_stdin_snapshot(
                    self.path,
                    value.line(),
                    &relative,
                    &mut self.case_text_cache,
                    &mut directives,
                ));
                self.stdin_jsonrpc_file = Some(relative);
                self.stdin_jsonrpc_workspace_file_uri_directives = directives;
            }
            "exit" => self.exit = Some(parse_i32(self.path, value)),
            "repeat" => self.repeat = parse_positive_usize(self.path, value),
            "source_errors" => {
                self.source_errors = parse_source_error_expectation(self.path, value)
            }
            _ => manifest_error(self.path, line_number, format!("unknown root key `{key}`")),
        }
    }

    pub(super) fn parse_requires_key(
        &mut self,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        match key {
            "jdk" => self.requires.jdk = parse_bool(self.path, value),
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown requires key `{key}`"),
            ),
        }
    }

    pub(super) fn parse_skip_key(
        &mut self,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        match key {
            "platforms" => {
                self.skip.platforms = parse_string_array(self.path, value)
                    .into_iter()
                    .map(|platform| parse_skip_platform(self.path, line_number, &platform))
                    .collect();
            }
            "reason" => self.skip.reason = Some(parse_string(self.path, value)),
            _ => manifest_error(self.path, line_number, format!("unknown skip key `{key}`")),
        }
    }

    pub(super) fn parse_tools_key(
        &mut self,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        match key {
            "java" => {
                self.tools
                    .set(ToolName::Java, parse_tool_availability(self.path, value));
            }
            "git" => {
                self.tools
                    .set(ToolName::Git, parse_tool_availability(self.path, value));
            }
            _ => manifest_error(self.path, line_number, format!("unknown tools key `{key}`")),
        }
    }
}
