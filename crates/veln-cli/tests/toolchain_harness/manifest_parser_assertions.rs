use super::*;

impl<'a> ManifestParser<'a> {
    pub(super) fn parse_json_assert_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        match key {
            "path" => self.json_assertions[index].path = parse_string(self.path, value),
            "equals" => {
                self.json_assertions[index].operation = Some(ValueAssertionOperation::Equals(
                    parse_manifest_json_value(self.path, value),
                ))
            }
            "equals_file" => {
                self.json_assertions[index].operation = Some(ValueAssertionOperation::EqualsFile(
                    JsonValue::String(self.case_text_cache.read(self.path, value)),
                ))
            }
            "equals_json_file" => {
                let text = self.case_text_cache.read(self.path, value);
                self.json_assertions[index].operation =
                    Some(ValueAssertionOperation::EqualsJsonFile(
                        parse_json(&text).unwrap_or_else(|error| {
                            manifest_error(
                                self.path,
                                line_number,
                                format!("invalid json_assert equals_json_file value: {error}"),
                            )
                        }),
                    ))
            }
            "contains" => {
                self.json_assertions[index].operation =
                    Some(parse_value_contains_operation(self.path, value));
            }
            "length" => {
                let context = value_assertion_context(
                    "json_assert",
                    index,
                    &self.json_assertions[index].path,
                    "length",
                );
                self.json_assertions[index].operation = Some(ValueAssertionOperation::Length(
                    parse_nonnegative_usize_with_context(self.path, value, &context),
                ));
            }
            "workspace_file_uri" => {
                let context = value_assertion_context(
                    "json_assert",
                    index,
                    &self.json_assertions[index].path,
                    "workspace_file_uri",
                );
                let relative = parse_string_with_context(self.path, value, &context);
                validate_workspace_file_uri_operand_with_context(
                    self.path,
                    line_number,
                    &relative,
                    Some(&context),
                );
                self.json_assertions[index].operation =
                    Some(ValueAssertionOperation::WorkspaceFileUri(relative));
            }
            "missing" => {
                let missing = parse_bool(self.path, value);
                debug_assert!(missing, "preflight rejects missing = false");
                self.json_assertions[index].operation = Some(ValueAssertionOperation::Missing);
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown json_assert key `{key}`"),
            ),
        }
    }

    pub(super) fn parse_result_value_assert_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        match key {
            "value_path" => {
                self.result_value_assertions[index].value_path = parse_string(self.path, value)
            }
            "path" => self.result_value_assertions[index].path = parse_string(self.path, value),
            "equals" => {
                self.result_value_assertions[index].operation = Some(
                    ValueAssertionOperation::Equals(parse_manifest_json_value(self.path, value)),
                )
            }
            "equals_file" => {
                self.result_value_assertions[index].operation =
                    Some(ValueAssertionOperation::EqualsFile(JsonValue::String(
                        self.case_text_cache.read(self.path, value),
                    )))
            }
            "equals_json_file" => {
                let text = self.case_text_cache.read(self.path, value);
                self.result_value_assertions[index].operation =
                    Some(ValueAssertionOperation::EqualsJsonFile(
                        parse_json(&text).unwrap_or_else(|error| {
                            manifest_error(
                                self.path,
                                line_number,
                                format!(
                                    "invalid result_value_assert equals_json_file value: {error}"
                                ),
                            )
                        }),
                    ))
            }
            "contains" => {
                self.result_value_assertions[index].operation =
                    Some(parse_value_contains_operation(self.path, value));
            }
            "length" => {
                let context = value_assertion_context(
                    "result_value_assert",
                    index,
                    &self.result_value_assertions[index].path,
                    "length",
                );
                self.result_value_assertions[index].operation =
                    Some(ValueAssertionOperation::Length(
                        parse_nonnegative_usize_with_context(self.path, value, &context),
                    ));
            }
            "workspace_file_uri" => {
                let context = value_assertion_context(
                    "result_value_assert",
                    index,
                    &self.result_value_assertions[index].path,
                    "workspace_file_uri",
                );
                let relative = parse_string_with_context(self.path, value, &context);
                validate_workspace_file_uri_operand_with_context(
                    self.path,
                    line_number,
                    &relative,
                    Some(&context),
                );
                self.result_value_assertions[index].operation =
                    Some(ValueAssertionOperation::WorkspaceFileUri(relative));
            }
            "missing" => {
                let missing = parse_bool(self.path, value);
                debug_assert!(missing, "preflight rejects missing = false");
                self.result_value_assertions[index].operation =
                    Some(ValueAssertionOperation::Missing);
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown result_value_assert key `{key}`"),
            ),
        }
    }

    pub(super) fn parse_lsp_assert_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        let assertion = &mut self.lsp_assertions[index];
        match key {
            "id" => {
                let id = parse_manifest_json_value(self.path, value);
                if !matches!(
                    id,
                    JsonValue::Null | JsonValue::Number(_) | JsonValue::String(_)
                ) && !matches!(
                    &id,
                    JsonValue::Decimal(raw) if is_json_integer_token(raw)
                ) {
                    manifest_error(
                        self.path,
                        line_number,
                        "lsp_assert `id` must be a JSON string, integer, or null",
                    );
                }
                assertion.id = Some(id);
            }
            "method" => assertion.method = Some(parse_string(self.path, value)),
            "occurrence" => assertion.occurrence = Some(parse_nonnegative_usize(self.path, value)),
            "path" => {
                assertion.path = parse_string(self.path, value);
                assertion.path_present = true;
                assertion.pointer_tokens = parse_json_pointer(
                    self.path,
                    line_number,
                    "lsp_assert",
                    index,
                    &assertion.path,
                );
            }
            "equals" => {
                assertion.operation_count += 1;
                assertion.operation = Some(RpcAssertionOperation::Equals(
                    parse_manifest_json_value(self.path, value),
                ));
            }
            "equals_file" => {
                assertion.operation_count += 1;
                assertion.operation = Some(RpcAssertionOperation::EqualsFileRef(
                    parse_case_text_reference(self.path, value, "lsp_assert", "equals_file"),
                ));
            }
            "equals_json_file" => {
                assertion.operation_count += 1;
                assertion.operation = Some(RpcAssertionOperation::EqualsJsonFileRef(
                    parse_case_text_reference(self.path, value, "lsp_assert", "equals_json_file"),
                ));
            }
            "contains" => {
                assertion.operation_count += 1;
                assertion.operation = Some(RpcAssertionOperation::Contains(parse_string(
                    self.path, value,
                )));
            }
            "length" => {
                assertion.operation_count += 1;
                let context = unresolved_assertion_operation_context("lsp_assert", index, "length");
                assertion.operation = Some(RpcAssertionOperation::Length(
                    parse_nonnegative_usize_with_context(self.path, value, &context),
                ));
            }
            "workspace_file_uri" => {
                assertion.operation_count += 1;
                let context = unresolved_assertion_operation_context(
                    "lsp_assert",
                    index,
                    "workspace_file_uri",
                );
                let relative = parse_string_with_context(self.path, value, &context);
                validate_workspace_file_uri_operand_with_context(
                    self.path,
                    line_number,
                    &relative,
                    Some(&context),
                );
                assertion.operation = Some(RpcAssertionOperation::WorkspaceFileUri(relative));
            }
            "missing" => {
                assertion.operation_count += 1;
                assertion.operation =
                    Some(RpcAssertionOperation::Missing(parse_bool(self.path, value)));
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown lsp_assert key `{key}`"),
            ),
        }
    }

    pub(super) fn parse_mcp_assert_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        let assertion = &mut self.mcp_assertions[index];
        match key {
            "id" => {
                let id = parse_manifest_json_value_allow_decimal(self.path, value);
                if !matches!(id, JsonValue::Number(_) | JsonValue::String(_))
                    && !matches!(
                        &id,
                        JsonValue::Decimal(raw) if is_json_integer_token(raw)
                    )
                {
                    manifest_error(
                        self.path,
                        line_number,
                        "mcp_assert `id` must be a JSON string or integer",
                    );
                }
                assertion.id = Some(id);
            }
            "path" => {
                assertion.path = parse_string(self.path, value);
                assertion.path_present = true;
                assertion.pointer_tokens = parse_json_pointer(
                    self.path,
                    line_number,
                    "mcp_assert",
                    index,
                    &assertion.path,
                );
            }
            "equals" => {
                assertion.operation_count += 1;
                assertion.operation = Some(RpcAssertionOperation::Equals(
                    parse_manifest_mcp_json_value(self.path, value),
                ));
            }
            "equals_file" => {
                assertion.operation_count += 1;
                assertion.operation = Some(RpcAssertionOperation::EqualsFileRef(
                    parse_case_text_reference(self.path, value, "mcp_assert", "equals_file"),
                ));
            }
            "equals_json_file" => {
                assertion.operation_count += 1;
                assertion.operation = Some(RpcAssertionOperation::EqualsJsonFileRef(
                    parse_case_text_reference(self.path, value, "mcp_assert", "equals_json_file"),
                ));
            }
            "contains" => {
                record_mcp_contains_assertion(assertion, self.path, value);
            }
            "length" => {
                assertion.operation_count += 1;
                let context = unresolved_assertion_operation_context("mcp_assert", index, "length");
                assertion.operation = Some(RpcAssertionOperation::Length(
                    parse_nonnegative_usize_with_context(self.path, value, &context),
                ));
            }
            "workspace_file_uri" => {
                assertion.operation_count += 1;
                let context = unresolved_assertion_operation_context(
                    "mcp_assert",
                    index,
                    "workspace_file_uri",
                );
                let relative = parse_string_with_context(self.path, value, &context);
                validate_workspace_file_uri_operand_with_context(
                    self.path,
                    line_number,
                    &relative,
                    Some(&context),
                );
                assertion.operation = Some(RpcAssertionOperation::WorkspaceFileUri(relative));
            }
            "missing" => {
                assertion.operation_count += 1;
                assertion.operation =
                    Some(RpcAssertionOperation::Missing(parse_bool(self.path, value)));
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown mcp_assert key `{key}`"),
            ),
        }
    }
}
