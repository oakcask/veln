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
            _ => {
                assertion.operation = Some(parse_rpc_assertion_operation(
                    self.path,
                    line_number,
                    "lsp_assert",
                    index,
                    key,
                    value,
                ));
                assertion.operation_count += 1;
            }
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
            _ => {
                assertion.operation = Some(parse_rpc_assertion_operation(
                    self.path,
                    line_number,
                    "mcp_assert",
                    index,
                    key,
                    value,
                ));
                assertion.operation_count += 1;
            }
        }
    }
}

fn parse_rpc_assertion_operation(
    path: &Path,
    line_number: usize,
    section: &str,
    index: usize,
    key: &str,
    value: &ManifestValue<'_>,
) -> RpcAssertionOperation {
    match key {
        "equals" => RpcAssertionOperation::Equals(parse_manifest_json_value(path, value)),
        "equals_file" => RpcAssertionOperation::EqualsFileRef(parse_case_text_reference(
            path, value, section, key,
        )),
        "equals_json_file" => RpcAssertionOperation::EqualsJsonFileRef(parse_case_text_reference(
            path, value, section, key,
        )),
        "contains" => RpcAssertionOperation::Contains(parse_string(path, value)),
        "length" => {
            let context = unresolved_assertion_operation_context(section, index, key);
            RpcAssertionOperation::Length(parse_nonnegative_usize_with_context(
                path, value, &context,
            ))
        }
        "workspace_file_uri" => {
            let context = unresolved_assertion_operation_context(section, index, key);
            let relative = parse_string_with_context(path, value, &context);
            validate_workspace_file_uri_operand_with_context(
                path,
                line_number,
                &relative,
                Some(&context),
            );
            RpcAssertionOperation::WorkspaceFileUri(relative)
        }
        "missing" => RpcAssertionOperation::Missing(parse_bool(path, value)),
        _ => manifest_error(path, line_number, format!("unknown {section} key `{key}`")),
    }
}
