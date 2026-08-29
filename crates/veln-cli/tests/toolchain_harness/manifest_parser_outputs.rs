use super::*;

impl<'a> ManifestParser<'a> {
    pub(super) fn parse_file_assert_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        match key {
            "path" => self.file_assertions[index].path = parse_string(self.path, value),
            "equals" => {
                self.file_assertions[index].operation_count += 1;
                self.file_assertions[index].equals = Some(parse_string(self.path, value));
            }
            "equals_file" => {
                self.file_assertions[index].operation_count += 1;
                self.file_assertions[index].equals =
                    Some(self.case_text_cache.read(self.path, value));
            }
            "missing" => {
                self.file_assertions[index].operation_count += 1;
                let missing = parse_bool(self.path, value);
                if !missing {
                    manifest_error(
                        self.path,
                        line_number,
                        format!("file_assert {index} `missing` must be true when present"),
                    );
                }
                self.file_assertions[index].missing = true;
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown file_assert key `{key}`"),
            ),
        }
    }

    pub(super) fn parse_diagnostic_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        match key {
            "id" => self.diagnostics[index].id = parse_string(self.path, value),
            "severity" => {
                self.diagnostics[index].severity = Some(parse_string(self.path, value));
            }
            "kind" => self.diagnostics[index].kind = Some(parse_string(self.path, value)),
            "message" => {
                self.diagnostics[index].message = Some(parse_string(self.path, value));
            }
            "message_file" => {
                self.diagnostics[index].message = Some(self.case_text_cache.read(self.path, value));
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown diagnostics key `{key}`"),
            ),
        }
    }

    pub(super) fn parse_diagnostic_span_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        let span = self.diagnostics[index]
            .span
            .as_mut()
            .expect("diagnostic span should exist");
        match key {
            "file" => span.file = Some(parse_string(self.path, value)),
            "line" => span.line = Some(parse_i64(self.path, value)),
            "column" => span.column = Some(parse_i64(self.path, value)),
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown diagnostics.span key `{key}`"),
            ),
        }
    }

    pub(super) fn parse_manifest_error_key(
        &mut self,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        let expectation = self
            .manifest_error
            .as_mut()
            .expect("manifest_error section should exist");
        match key {
            "contains" => {
                expectation
                    .contains
                    .extend(parse_string_array(self.path, value));
            }
            "contains_file" => {
                expectation
                    .contains
                    .push(self.case_text_cache.read(self.path, value));
            }
            "contains_files" => {
                expectation
                    .contains
                    .extend(self.case_text_cache.read_many(self.path, value));
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown manifest_error key `{key}`"),
            ),
        }
    }

    pub(super) fn parse_binary_fixture_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        let fixture = &mut self.binary_fixtures[index];
        match key {
            "name" => fixture.name = parse_string(self.path, value),
            "schema" => fixture.schema = Some(parse_string(self.path, value)),
            "hex" => {
                fixture.bytes = Some(parse_binary_fixture_hex(self.path, value));
            }
            "consumed" => {
                fixture.consumed = Some(parse_nonnegative_usize(self.path, value));
            }
            "error" => fixture.error = Some(parse_string(self.path, value)),
            "diagnostic_id" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .diagnostic_id = Some(parse_string(self.path, value));
            }
            "byte_offset" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .byte_offset = Some(parse_nonnegative_usize(self.path, value));
            }
            "expected_count" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .expected_count = Some(parse_nonnegative_usize(self.path, value));
            }
            "available_count" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .available_count = Some(parse_nonnegative_usize(self.path, value));
            }
            "readiness" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .readiness = Some(parse_string(self.path, value));
            }
            "field_path" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .field_path = Some(parse_manifest_json_value(self.path, value));
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown binary_fixture key `{key}`"),
            ),
        }
    }

    pub(super) fn parse_output_chunk_list_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        let chunks = &mut self.output_chunk_lists[index];
        match key {
            "name" => chunks.name = parse_string(self.path, value),
            "chunks" => {
                chunks.chunks = Some(parse_binary_fixture_hex_array(self.path, value));
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown output_chunk_list key `{key}`"),
            ),
        }
    }

    pub(super) fn finish(mut self) -> CaseManifest {
        let path = self.path;
        let mut case_text_cache = std::mem::take(&mut self.case_text_cache);
        if self.stdin_operand_count > 1 {
            manifest_error(
                path,
                0,
                "root invocation needs at most one of `stdin`, `stdin_file`, or `stdin_jsonrpc_file`",
            );
        }
        let mut manifest = CaseManifest {
            invocation: CaseInvocation {
                command: self
                    .command
                    .unwrap_or_else(|| manifest_error(self.path, 0, "missing `command`")),
                cwd: self.cwd,
                stdin: self.stdin,
                stdin_jsonrpc_file: self.stdin_jsonrpc_file,
                stdin_jsonrpc_workspace_file_uri_directives: self
                    .stdin_jsonrpc_workspace_file_uri_directives,
                repeat: self.repeat,
                env: self.env,
            },
            expectations: CaseExpectations {
                exit: self
                    .exit
                    .unwrap_or_else(|| manifest_error(self.path, 0, "missing `exit`")),
                stdout: self.stdout,
                stderr: self.stderr,
                help: self.help,
                json_assertions: self.json_assertions,
                result_value_assertions: self.result_value_assertions,
                lsp_assertions: self.lsp_assertions,
                mcp_assertions: self.mcp_assertions,
                file_assertions: self.file_assertions,
                diagnostics: self.diagnostics,
                binary_fixtures: self.binary_fixtures,
                output_chunk_lists: self.output_chunk_lists,
            },
            source_errors: self.source_errors,
            manifest_error: self.manifest_error,
            tools: self.tools,
            requires: self.requires,
            skip: self.skip,
        };

        manifest.validate(path);
        resolve_lsp_mcp_file_backed_assertions(
            path,
            &mut manifest.expectations,
            &mut case_text_cache,
        );
        manifest
    }
}
