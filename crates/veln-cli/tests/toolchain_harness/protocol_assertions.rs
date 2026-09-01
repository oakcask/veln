use super::*;

pub(super) fn assert_lsp_assertions(
    context: &CaseRunContext<'_>,
    stdout: &str,
    assertions: &[LspAssertion],
) {
    assert_lsp_assertions_in_workspace(context, stdout, assertions, Path::new("."));
}

pub(super) fn assert_lsp_assertions_in_workspace(
    context: &CaseRunContext<'_>,
    stdout: &str,
    assertions: &[LspAssertion],
    project_root: &Path,
) {
    if assertions.is_empty() {
        return;
    }
    let messages = decode_lsp_stdout(stdout).unwrap_or_else(|error| {
        let selectors = assertions
            .iter()
            .map(LspAssertion::selector)
            .collect::<Vec<_>>()
            .join(", ");
        panic!(
            "{}: decoded LSP stream failed for {selectors}: {error}",
            context.label()
        )
    });
    let mut failures = Vec::new();
    for (index, assertion) in assertions.iter().enumerate() {
        if let Err(error) = evaluate_lsp_assertion_in_workspace(&messages, assertion, project_root)
        {
            failures.push(format!(
                "{}: lsp_assert {index} {} path {:?}: {error}",
                context.label(),
                assertion.selector(),
                assertion.path
            ));
        }
    }
    if !failures.is_empty() {
        panic!("{}", failures.join("\n"));
    }
}

pub(super) fn assert_mcp_assertions(
    context: &CaseRunContext<'_>,
    stdout: &str,
    assertions: &[McpAssertion],
    project_root: &Path,
) {
    if assertions.is_empty() {
        return;
    }
    let messages = decode_mcp_stdout(stdout).unwrap_or_else(|error| {
        let selectors = assertions
            .iter()
            .map(McpAssertion::selector)
            .collect::<Vec<_>>()
            .join(", ");
        panic!(
            "{}: decoded MCP JSONL stream failed for {selectors}: {error}",
            context.label()
        )
    });
    let mut failures = Vec::new();
    for (index, assertion) in assertions.iter().enumerate() {
        if let Err(error) = evaluate_mcp_assertion(&messages, assertion, project_root) {
            failures.push(format!(
                "{}: mcp_assert {index} {} path {:?}: {error}",
                context.label(),
                assertion.selector(),
                assertion.path
            ));
        }
    }
    if !failures.is_empty() {
        panic!("{}", failures.join("\n"));
    }
}

pub(super) fn decode_mcp_stdout(stdout: &str) -> Result<Vec<JsonValue>, String> {
    let mut messages = Vec::new();
    for (index, line) in stdout.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let message = parse_json(line)
            .map_err(|error| format!("line {} is invalid JSON: {error}", index + 1))?;
        if !matches!(message, JsonValue::Object(_)) {
            return Err(format!("line {} is not a JSON-RPC object", index + 1));
        }
        messages.push(message);
    }
    Ok(messages)
}

pub(super) fn decode_lsp_stdout(stdout: &str) -> Result<Vec<JsonValue>, String> {
    let bytes = stdout.as_bytes();
    let mut offset = 0usize;
    let mut messages = Vec::new();
    while offset < bytes.len() {
        let missing_delimiter_error = if messages.is_empty() {
            "malformed or partial framing"
        } else {
            "trailing bytes"
        };
        let (message, next_offset) = decode_lsp_frame(bytes, offset, missing_delimiter_error)?;
        messages.push(message);
        offset = next_offset;
    }

    validate_unique_lsp_response_ids(&messages)?;
    Ok(messages)
}

pub(super) fn decode_lsp_frame(
    bytes: &[u8],
    offset: usize,
    missing_delimiter_error: &str,
) -> Result<(JsonValue, usize), String> {
    let (body_start, content_length) = decode_lsp_header(bytes, offset, missing_delimiter_error)?;
    let body_end = body_start
        .checked_add(content_length)
        .ok_or_else(|| format!("Content-Length overflow at byte offset {offset}"))?;
    if body_end > bytes.len() {
        return Err(format!(
            "partial frame body at byte offset {body_start}: expected {content_length} bytes, found {}",
            bytes.len() - body_start
        ));
    }
    let body = std::str::from_utf8(&bytes[body_start..body_end])
        .map_err(|_| format!("frame body at byte offset {body_start} is not UTF-8"))?;
    let message = parse_json(body).map_err(|error| {
        format!("frame body at byte offset {body_start} is invalid JSON: {error}")
    })?;
    if !matches!(message, JsonValue::Object(_)) {
        return Err(format!(
            "frame body at byte offset {body_start} is not a JSON-RPC object"
        ));
    }
    Ok((message, body_end))
}

pub(super) fn decode_lsp_header(
    bytes: &[u8],
    offset: usize,
    missing_delimiter_error: &str,
) -> Result<(usize, usize), String> {
    let Some(header_end_relative) = find_bytes(&bytes[offset..], b"\r\n\r\n") else {
        return Err(format!("{missing_delimiter_error} at byte offset {offset}"));
    };
    let header_end = offset + header_end_relative;
    let header = std::str::from_utf8(&bytes[offset..header_end])
        .map_err(|_| format!("malformed frame header at byte offset {offset}"))?;
    let mut content_length = None;
    for line in header.split("\r\n") {
        if let Some(raw) = line.strip_prefix("Content-Length:") {
            if content_length.is_some() {
                return Err(format!(
                    "duplicate Content-Length header at byte offset {offset}"
                ));
            }
            let raw = raw.trim();
            content_length =
                Some(raw.parse::<usize>().map_err(|_| {
                    format!("invalid Content-Length `{raw}` at byte offset {offset}")
                })?);
        }
    }
    let content_length = content_length
        .ok_or_else(|| format!("missing Content-Length header at byte offset {offset}"))?;
    Ok((header_end + 4, content_length))
}

pub(super) fn validate_unique_lsp_response_ids(messages: &[JsonValue]) -> Result<(), String> {
    let mut response_ids = Vec::<&JsonValue>::new();
    for message in messages {
        if is_lsp_response(message) {
            let id = message
                .object_field("id")
                .expect("response classification requires id");
            if response_ids.contains(&id) {
                return Err(format!(
                    "duplicate response identifier {}",
                    id.to_compact_string()
                ));
            }
            response_ids.push(id);
        }
    }
    Ok(())
}

pub(super) fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub(super) fn is_lsp_response(message: &JsonValue) -> bool {
    message.object_field("id").is_some()
        && message.object_field("method").is_none()
        && (message.object_field("result").is_some() || message.object_field("error").is_some())
}

pub(super) fn evaluate_lsp_assertion(
    messages: &[JsonValue],
    assertion: &LspAssertion,
) -> Result<(), String> {
    evaluate_lsp_assertion_in_workspace(messages, assertion, Path::new("."))
}

pub(super) fn evaluate_lsp_assertion_in_workspace(
    messages: &[JsonValue],
    assertion: &LspAssertion,
    project_root: &Path,
) -> Result<(), String> {
    let selected = if let Some(id) = &assertion.id {
        messages
            .iter()
            .find(|message| is_lsp_response(message) && message.object_field("id") == Some(id))
            .ok_or_else(|| "selected response was not found".to_string())?
    } else {
        let method = assertion
            .method
            .as_deref()
            .expect("validated method selector");
        messages
            .iter()
            .filter(|message| {
                message.object_field("id").is_none()
                    && message.object_field("method").and_then(JsonValue::as_str) == Some(method)
            })
            .nth(assertion.occurrence.unwrap_or(0))
            .ok_or_else(|| "selected notification was not found".to_string())?
    };

    let pointer_tokens =
        materialize_workspace_file_uri_pointer_tokens(&assertion.pointer_tokens, project_root)?;
    evaluate_protocol_pointer_result(
        json_pointer(selected, &pointer_tokens),
        assertion
            .operation
            .as_ref()
            .expect("validated LSP assertion operation"),
        project_root,
    )
}

pub(super) fn evaluate_protocol_pointer_result(
    result: JsonPointerResult<'_>,
    operation: &RpcAssertionOperation,
    project_root: &Path,
) -> Result<(), String> {
    match result {
        JsonPointerResult::Missing => {
            if matches!(operation, RpcAssertionOperation::Missing(true)) {
                Ok(())
            } else {
                Err("selected JSON path was not found".to_string())
            }
        }
        JsonPointerResult::Invalid(reason) => Err(format!("invalid traversal: {reason}")),
        JsonPointerResult::Found(actual) => match operation {
            RpcAssertionOperation::Equals(expected) => expect_json_value(actual, expected),
            RpcAssertionOperation::EqualsFile(expected) => {
                expect_string_equals_file(actual, expected)
            }
            RpcAssertionOperation::EqualsFileRef(_) => {
                unreachable!("manifest finish resolves protocol equals_file operands")
            }
            RpcAssertionOperation::EqualsJsonFile(expected) => expect_json_value(actual, expected),
            RpcAssertionOperation::EqualsJsonFileRef(_) => {
                unreachable!("manifest finish resolves protocol equals_json_file operands")
            }
            RpcAssertionOperation::Contains(expected) => expect_string_contains(actual, expected),
            RpcAssertionOperation::Length(expected) => expect_array_length(actual, *expected),
            RpcAssertionOperation::Missing(true) => {
                Err("selected JSON path exists but should be missing".to_string())
            }
            RpcAssertionOperation::Missing(false) => {
                unreachable!("validated missing operation")
            }
            RpcAssertionOperation::WorkspaceFileUri(relative) => {
                expect_workspace_file_uri(actual, project_root, relative)
            }
        },
    }
}

pub(super) fn evaluate_mcp_assertion(
    messages: &[JsonValue],
    assertion: &McpAssertion,
    project_root: &Path,
) -> Result<(), String> {
    let id = assertion.id.as_ref().expect("validated MCP id");
    let selected = select_mcp_response(messages, id)?;
    let pointer_tokens =
        materialize_workspace_file_uri_pointer_tokens(&assertion.pointer_tokens, project_root)?;
    evaluate_protocol_pointer_result(
        json_pointer(selected, &pointer_tokens),
        assertion
            .operation
            .as_ref()
            .expect("validated MCP assertion operation"),
        project_root,
    )
}

pub(super) const WORKSPACE_FILE_URI_POINTER_TOKEN_PREFIX: &str = "$workspace_file_uri:";

pub(super) fn materialize_workspace_file_uri_pointer_tokens(
    tokens: &[String],
    project_root: &Path,
) -> Result<Vec<String>, String> {
    tokens
        .iter()
        .map(|token| {
            if let Some(relative) = token.strip_prefix(WORKSPACE_FILE_URI_POINTER_TOKEN_PREFIX) {
                workspace_file_uri(project_root, relative)
            } else {
                Ok(token.clone())
            }
        })
        .collect()
}

pub(super) fn select_mcp_response<'a>(
    messages: &'a [JsonValue],
    id: &JsonValue,
) -> Result<&'a JsonValue, String> {
    let matches = messages
        .iter()
        .filter(|message| {
            message.object_field("id") == Some(id)
                && message.object_field("method").is_none()
                && (message.object_field("result").is_some()
                    || message.object_field("error").is_some())
        })
        .collect::<Vec<_>>();
    let selected = match matches.as_slice() {
        [selected] => *selected,
        [] => return Err("selected response was not found".to_string()),
        _ => {
            return Err(format!(
                "selected response id {} matched {} responses",
                id.to_compact_string(),
                matches.len()
            ));
        }
    };
    Ok(selected)
}

pub(super) fn expect_json_value(actual: &JsonValue, expected: &JsonValue) -> Result<(), String> {
    if json_values_equal(actual, expected) {
        Ok(())
    } else {
        Err(format!(
            "value mismatch: expected {}, got {}",
            expected.to_compact_string(),
            actual.to_compact_string()
        ))
    }
}

pub(super) fn expect_string_equals_file(actual: &JsonValue, expected: &str) -> Result<(), String> {
    let actual = actual
        .as_str()
        .ok_or_else(|| "equals_file requires a selected JSON string".to_string())?;
    if actual == expected {
        Ok(())
    } else {
        Err("string does not equal the expected file contents".to_string())
    }
}

pub(super) fn expect_string_contains(actual: &JsonValue, expected: &str) -> Result<(), String> {
    let actual = actual
        .as_str()
        .ok_or_else(|| "contains requires a selected JSON string".to_string())?;
    if actual.contains(expected) {
        Ok(())
    } else {
        Err(format!("string does not contain {expected:?}"))
    }
}

pub(super) fn expect_array_length(actual: &JsonValue, expected: usize) -> Result<(), String> {
    let actual = actual
        .as_array()
        .ok_or_else(|| "length requires a selected JSON array".to_string())?;
    if actual.len() == expected {
        Ok(())
    } else {
        Err(format!(
            "array length mismatch: expected {expected}, got {}",
            actual.len()
        ))
    }
}

pub(super) fn expect_workspace_file_uri(
    actual: &JsonValue,
    project_root: &Path,
    relative: &str,
) -> Result<(), String> {
    let expected = workspace_file_uri(project_root, relative)?;
    let actual = actual
        .as_str()
        .ok_or_else(|| "workspace_file_uri requires a selected JSON string".to_string())?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "workspace URI mismatch: expected {expected}, got {actual}"
        ))
    }
}

pub(super) fn json_values_equal(left: &JsonValue, right: &JsonValue) -> bool {
    match (left, right) {
        (JsonValue::Null, JsonValue::Null) => true,
        (JsonValue::Bool(left), JsonValue::Bool(right)) => left == right,
        (JsonValue::Number(left), JsonValue::Number(right)) => left == right,
        (JsonValue::Number(left), JsonValue::Decimal(right))
        | (JsonValue::Decimal(right), JsonValue::Number(left)) => left.to_string() == *right,
        (JsonValue::Decimal(left), JsonValue::Decimal(right)) => left == right,
        (JsonValue::String(left), JsonValue::String(right)) => left == right,
        (JsonValue::Array(left), JsonValue::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| json_values_equal(left, right))
        }
        (JsonValue::Object(left), JsonValue::Object(right)) => {
            if left.len() != right.len() {
                return false;
            }
            let mut matched = vec![false; right.len()];
            left.iter().all(|(key, left_value)| {
                let Some(index) =
                    right
                        .iter()
                        .enumerate()
                        .position(|(index, (right_key, right_value))| {
                            !matched[index]
                                && right_key == key
                                && json_values_equal(left_value, right_value)
                        })
                else {
                    return false;
                };
                matched[index] = true;
                true
            })
        }
        _ => false,
    }
}

pub(super) fn validate_workspace_file_uri_operand(path: &Path, line_number: usize, relative: &str) {
    validate_workspace_file_uri_operand_with_context(path, line_number, relative, None);
}

pub(super) fn validate_workspace_file_uri_operand_with_context(
    path: &Path,
    line_number: usize,
    relative: &str,
    context: Option<&str>,
) {
    let case_dir = path.parent().unwrap_or_else(|| Path::new("."));
    if let Err(error) = validate_workspace_relative_file(case_dir, relative) {
        manifest_error(
            path,
            line_number,
            match context {
                Some(context) => format!("{context}: {error}"),
                None => error,
            },
        );
    }
}

pub(super) fn workspace_file_uri(project_root: &Path, relative: &str) -> Result<String, String> {
    validate_workspace_relative_file(project_root, relative)?;
    let path = project_root.join(relative);
    if is_link_like_metadata(
        &path
            .symlink_metadata()
            .map_err(|error| format!("workspace file `{relative}` is not available: {error}"))?,
    ) {
        return Err(format!(
            "workspace file `{relative}` must not be a link-like entry"
        ));
    }
    let metadata = path
        .symlink_metadata()
        .map_err(|error| format!("workspace file `{relative}` is not available: {error}"))?;
    if is_link_like_metadata(&metadata) {
        return Err(format!(
            "workspace file `{relative}` must not be a link-like entry"
        ));
    }
    if !metadata.is_file() {
        return Err(format!(
            "workspace file `{relative}` must be a regular file"
        ));
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("workspace file `{relative}` cannot be canonicalized: {error}"))?;
    let canonical_root = project_root
        .canonicalize()
        .map_err(|error| format!("workspace root cannot be canonicalized: {error}"))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(format!(
            "workspace file `{relative}` must stay inside the canonical workspace root"
        ));
    }
    Ok(path_to_file_uri(&canonical))
}

pub(super) fn validate_workspace_relative_file(base: &Path, relative: &str) -> Result<(), String> {
    let path = Path::new(relative);
    if relative.is_empty() || path.is_absolute() || relative.contains('\\') {
        return Err(format!(
            "workspace_file_uri `{relative}` must be a nonempty workspace-relative path"
        ));
    }
    if relative
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(format!(
            "workspace_file_uri `{relative}` must not contain `.`, `..`, empty, root, or prefix segments"
        ));
    }
    for component in path.components() {
        match component {
            std::path::Component::Normal(segment) if !segment.is_empty() => {}
            _ => {
                return Err(format!(
                    "workspace_file_uri `{relative}` must not contain `.`, `..`, empty, root, or prefix segments"
                ));
            }
        }
    }
    let mut full = base.to_path_buf();
    for component in path.components() {
        full.push(component);
        let metadata = full.symlink_metadata().map_err(|error| {
            format!("workspace_file_uri `{relative}` must name an existing regular file: {error}")
        })?;
        if is_link_like_metadata(&metadata) {
            return Err(format!(
                "workspace_file_uri `{relative}` must not name a link-like entry"
            ));
        }
    }
    if !full
        .metadata()
        .map_err(|error| {
            format!("workspace_file_uri `{relative}` must name an existing regular file: {error}")
        })?
        .is_file()
    {
        return Err(format!(
            "workspace_file_uri `{relative}` must name an existing regular file"
        ));
    }
    Ok(())
}

pub(super) fn path_to_file_uri(path: &Path) -> String {
    #[cfg(unix)]
    let bytes = path.as_os_str().as_bytes();
    #[cfg(not(unix))]
    let path_text = path.to_string_lossy();
    #[cfg(not(unix))]
    let bytes = path_text.as_bytes();
    let mut encoded = String::new();
    for &byte in bytes {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                encoded.push(byte as char);
            }
            byte => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    format!("file://{encoded}")
}
