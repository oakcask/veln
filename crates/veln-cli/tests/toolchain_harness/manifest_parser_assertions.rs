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
            _ => {
                let assertion_path = self.json_assertions[index].path.clone();
                self.json_assertions[index].operation = Some(self.parse_value_assertion_operation(
                    line_number,
                    "json_assert",
                    index,
                    &assertion_path,
                    key,
                    value,
                ));
            }
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
            _ => {
                let assertion_path = self.result_value_assertions[index].path.clone();
                self.result_value_assertions[index].operation =
                    Some(self.parse_value_assertion_operation(
                        line_number,
                        "result_value_assert",
                        index,
                        &assertion_path,
                        key,
                        value,
                    ));
            }
        }
    }

    fn parse_value_assertion_operation(
        &mut self,
        line_number: usize,
        section: &str,
        index: usize,
        assertion_path: &str,
        key: &str,
        value: &ManifestValue<'_>,
    ) -> ValueAssertionOperation {
        match key {
            "equals" => {
                ValueAssertionOperation::Equals(parse_manifest_json_value(self.path, value))
            }
            "equals_file" => ValueAssertionOperation::EqualsFile(JsonValue::String(
                self.case_text_cache.read(self.path, value),
            )),
            "equals_json_file" => {
                let text = self.case_text_cache.read(self.path, value);
                ValueAssertionOperation::EqualsJsonFile(parse_json(&text).unwrap_or_else(|error| {
                    manifest_error(
                        self.path,
                        line_number,
                        format!("invalid {section} equals_json_file value: {error}"),
                    )
                }))
            }
            "contains" => parse_value_contains_operation(self.path, value),
            "length" => {
                let context = value_assertion_context(section, index, assertion_path, "length");
                ValueAssertionOperation::Length(parse_nonnegative_usize_with_context(
                    self.path, value, &context,
                ))
            }
            "workspace_file_uri" => {
                let context =
                    value_assertion_context(section, index, assertion_path, "workspace_file_uri");
                let relative = parse_string_with_context(self.path, value, &context);
                validate_workspace_file_uri_operand_with_context(
                    self.path,
                    line_number,
                    &relative,
                    Some(&context),
                );
                ValueAssertionOperation::WorkspaceFileUri(relative)
            }
            "missing" => {
                let missing = parse_bool(self.path, value);
                debug_assert!(missing, "preflight rejects missing = false");
                ValueAssertionOperation::Missing
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown {section} key `{key}`"),
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
        "equals_response" if section == "mcp_assert" => RpcAssertionOperation::EqualsResponse(
            parse_mcp_response_reference(path, line_number, index, value),
        ),
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

fn parse_mcp_response_reference(
    path: &Path,
    line_number: usize,
    index: usize,
    value: &ManifestValue<'_>,
) -> McpResponseReference {
    let operand = parse_manifest_json_value_allow_decimal(path, value);
    let JsonValue::Object(entries) = &operand else {
        manifest_error(
            path,
            line_number,
            "mcp_assert `equals_response` must be an object with `id` and `path`",
        );
    };
    if entries.len() != 2
        || entries.iter().filter(|(key, _)| key == "id").count() != 1
        || entries.iter().filter(|(key, _)| key == "path").count() != 1
    {
        manifest_error(
            path,
            line_number,
            "mcp_assert `equals_response` must contain exactly `id` and `path`",
        );
    }
    let id = operand
        .object_field("id")
        .expect("validated equals_response id")
        .clone();
    if !matches!(id, JsonValue::Number(_) | JsonValue::String(_))
        && !matches!(&id, JsonValue::Decimal(raw) if is_json_integer_token(raw))
    {
        manifest_error(
            path,
            line_number,
            "mcp_assert `equals_response.id` must be a JSON string or integer",
        );
    }
    let response_path = operand
        .object_field("path")
        .and_then(JsonValue::as_str)
        .unwrap_or_else(|| {
            manifest_error(
                path,
                line_number,
                "mcp_assert `equals_response.path` must be a string",
            )
        })
        .to_string();
    let pointer_tokens = parse_json_pointer(
        path,
        line_number,
        "mcp_assert equals_response",
        index,
        &response_path,
    );
    McpResponseReference {
        id,
        path: response_path,
        pointer_tokens,
    }
}
