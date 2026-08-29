use super::*;

#[derive(Debug, Default)]
pub(super) struct CaseTextCache {
    pub(super) snapshots: BTreeMap<PathBuf, String>,
}

impl CaseTextCache {
    pub(super) fn read_many(&mut self, path: &Path, value: &ManifestValue<'_>) -> Vec<String> {
        parse_string_array(path, value)
            .into_iter()
            .map(|relative| self.read_path(path, value.line(), &relative))
            .collect()
    }

    pub(super) fn read(&mut self, path: &Path, value: &ManifestValue<'_>) -> String {
        let relative = parse_string(path, value);
        self.read_path(path, value.line(), &relative)
    }

    pub(super) fn read_path(&mut self, path: &Path, line_number: usize, relative: &str) -> String {
        self.read_path_with_context(path, line_number, relative, None)
    }

    pub(super) fn read_path_with_context(
        &mut self,
        path: &Path,
        line_number: usize,
        relative: &str,
        context: Option<&str>,
    ) -> String {
        let relative_path = validate_case_file_reference(path, line_number, relative, context);
        if let Some(snapshot) = self.snapshots.get(&relative_path) {
            return snapshot.clone();
        }
        let text = read_case_text_file_path(path, line_number, relative, &relative_path, context);
        self.snapshots.insert(relative_path, text.clone());
        text
    }
}

pub(super) fn parse_case_text_reference(
    path: &Path,
    value: &ManifestValue<'_>,
    section: &str,
    operation: &str,
) -> CaseTextReference {
    if !value.is_string() {
        manifest_error(
            path,
            value.line(),
            format!("{section} `{operation}` must be a string case file reference"),
        );
    }
    CaseTextReference {
        line_number: value.line(),
        relative: parse_string(path, value),
    }
}

pub(super) fn load_jsonrpc_stdin_snapshot(
    manifest_path: &Path,
    line_number: usize,
    relative: &str,
    case_text_cache: &mut CaseTextCache,
    workspace_file_uri_directives: &mut Vec<WorkspaceFileUriDirective>,
) -> String {
    load_jsonrpc_stdin(
        manifest_path,
        line_number,
        relative,
        case_text_cache,
        workspace_file_uri_directives,
    )
}

pub(super) fn load_jsonrpc_stdin(
    manifest_path: &Path,
    line_number: usize,
    relative: &str,
    case_text_cache: &mut CaseTextCache,
    workspace_file_uri_directives: &mut Vec<WorkspaceFileUriDirective>,
) -> String {
    let text = case_text_cache.read_path(manifest_path, line_number, relative);
    let fixture = parse_json(&text).unwrap_or_else(|error| {
        let message_context = jsonrpc_parse_error_message_context(&text, error.offset)
            .map(|index| format!(" message {index}"))
            .unwrap_or_default();
        manifest_error(
            manifest_path,
            line_number,
            format!("invalid JSON-RPC fixture `{relative}`{message_context}: {error}"),
        )
    });
    let JsonValue::Array(messages) = fixture else {
        manifest_error(
            manifest_path,
            line_number,
            format!("JSON-RPC fixture `{relative}` root must be an array"),
        );
    };

    let mut framed = String::new();
    for (index, mut message) in messages.into_iter().enumerate() {
        let position = format!("$[{index}]");
        let mut context = JsonrpcDirectiveExpansion {
            manifest_path,
            line_number,
            message_index: index,
            case_text_cache,
            workspace_file_uri_directives,
        };
        expand_case_text_directives(
            &mut context,
            &position,
            &mut Vec::new(),
            &mut Vec::new(),
            &mut message,
        );
        validate_jsonrpc_input_message(manifest_path, line_number, index, &message);
        let body = message.to_compact_string();
        let length = body.len();
        framed.push_str(&format!("Content-Length: {length}\r\n\r\n{body}"));
    }
    framed
}

pub(super) struct JsonrpcDirectiveExpansion<'a> {
    pub(super) manifest_path: &'a Path,
    pub(super) line_number: usize,
    pub(super) message_index: usize,
    pub(super) case_text_cache: &'a mut CaseTextCache,
    pub(super) workspace_file_uri_directives: &'a mut Vec<WorkspaceFileUriDirective>,
}

pub(super) fn expand_case_text_directives(
    context: &mut JsonrpcDirectiveExpansion<'_>,
    position: &str,
    pointer_tokens: &mut Vec<String>,
    pointer_route: &mut Vec<JsonPointerRouteSegment>,
    value: &mut JsonValue,
) {
    match value {
        JsonValue::Array(values) => {
            for (index, value) in values.iter_mut().enumerate() {
                pointer_tokens.push(index.to_string());
                pointer_route.push(JsonPointerRouteSegment::ArrayIndex(index));
                expand_case_text_directives(
                    context,
                    &format!("{position}[{index}]"),
                    pointer_tokens,
                    pointer_route,
                    value,
                );
                pointer_tokens.pop();
                pointer_route.pop();
            }
        }
        JsonValue::Object(entries) => {
            if let Some((_, directive)) =
                entries.iter().find(|(key, _)| key == "$workspace_file_uri")
            {
                if entries.len() != 1 {
                    jsonrpc_fixture_error(
                        context.manifest_path,
                        context.line_number,
                        context.message_index,
                        position,
                        "`$workspace_file_uri` directive object must contain no other members",
                    );
                }
                let JsonValue::String(relative) = directive else {
                    jsonrpc_fixture_error(
                        context.manifest_path,
                        context.line_number,
                        context.message_index,
                        position,
                        "`$workspace_file_uri` directive value must be a string",
                    );
                };
                validate_workspace_file_uri_operand(
                    context.manifest_path,
                    context.line_number,
                    relative,
                );
                context
                    .workspace_file_uri_directives
                    .push(WorkspaceFileUriDirective {
                        message_index: context.message_index,
                        pointer_route: pointer_route.clone(),
                        relative: relative.clone(),
                    });
                *value = JsonValue::String(workspace_file_uri_marker(relative));
                return;
            }
            if let Some((_, directive)) = entries.iter().find(|(key, _)| key == "$case_text") {
                if entries.len() != 1 {
                    jsonrpc_fixture_error(
                        context.manifest_path,
                        context.line_number,
                        context.message_index,
                        position,
                        "`$case_text` directive object must contain no other members",
                    );
                }
                let JsonValue::String(relative) = directive else {
                    jsonrpc_fixture_error(
                        context.manifest_path,
                        context.line_number,
                        context.message_index,
                        position,
                        "`$case_text` directive value must be a string",
                    );
                };
                let replacement = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    context.case_text_cache.read_path(
                        context.manifest_path,
                        context.line_number,
                        relative,
                    )
                }))
                .unwrap_or_else(|panic| {
                    jsonrpc_fixture_error(
                        context.manifest_path,
                        context.line_number,
                        context.message_index,
                        position,
                        &format!("case-text reference failed: {}", panic_message(panic)),
                    )
                });
                *value = JsonValue::String(replacement);
                return;
            }
            let mut seen_keys = BTreeMap::<String, usize>::new();
            for (key, value) in entries {
                let occurrence = *seen_keys.get(key).unwrap_or(&0);
                seen_keys.insert(key.clone(), occurrence + 1);
                pointer_tokens.push(key.clone());
                pointer_route.push(JsonPointerRouteSegment::ObjectMember {
                    key: key.clone(),
                    occurrence,
                });
                expand_case_text_directives(
                    context,
                    &format!("{position}.{}", escape_json_position_key(key)),
                    pointer_tokens,
                    pointer_route,
                    value,
                );
                pointer_tokens.pop();
                pointer_route.pop();
            }
        }
        _ => {}
    }
}

pub(super) const WORKSPACE_FILE_URI_MARKER: &str = "veln-harness-workspace-file-uri:";

pub(super) fn workspace_file_uri_marker(relative: &str) -> String {
    let mut marker = WORKSPACE_FILE_URI_MARKER.to_string();
    for byte in relative.bytes() {
        marker.push_str(&format!("{byte:02x}"));
    }
    marker
}

pub(super) fn materialize_jsonrpc_workspace_file_uri_directives(
    input: &str,
    directives: &[WorkspaceFileUriDirective],
    project_root: &Path,
) -> String {
    if directives.is_empty() {
        return input.to_string();
    }
    let mut messages = decode_lsp_stdout(input)
        .unwrap_or_else(|error| panic!("workspace URI directive input failed to decode: {error}"));
    for directive in directives {
        let Some(message) = messages.get_mut(directive.message_index) else {
            panic!(
                "workspace URI directive references missing message {}",
                directive.message_index
            );
        };
        let target = json_pointer_route_mut(message, &directive.pointer_route)
            .unwrap_or_else(|| panic!("workspace URI directive path was not found"));
        let uri = workspace_file_uri(project_root, &directive.relative)
            .unwrap_or_else(|error| panic!("workspace URI directive failed: {error}"));
        *target = JsonValue::String(uri);
    }
    messages
        .iter()
        .map(|message| lsp_frame(&message.to_compact_string()))
        .collect()
}

pub(super) fn escape_json_position_key(key: &str) -> String {
    if key
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        key.to_string()
    } else {
        format!(
            "[{}]",
            JsonValue::String(key.to_string()).to_compact_string()
        )
    }
}

pub(super) fn validate_jsonrpc_input_message(
    manifest_path: &Path,
    line_number: usize,
    index: usize,
    message: &JsonValue,
) {
    let JsonValue::Object(entries) = message else {
        jsonrpc_fixture_error(
            manifest_path,
            line_number,
            index,
            &format!("$[{index}]"),
            "message must be an object",
        );
    };
    let field = |name: &str| {
        entries
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    };
    if field("result").is_some() || field("error").is_some() {
        jsonrpc_fixture_error(
            manifest_path,
            line_number,
            index,
            &format!("$[{index}]"),
            "request or notification must not contain `result` or `error`",
        );
    }
    for name in ["jsonrpc", "method", "id", "params"] {
        let count = entries.iter().filter(|(key, _)| key == name).count();
        if count > 1 {
            jsonrpc_fixture_error(
                manifest_path,
                line_number,
                index,
                &format!("$[{index}].{name}"),
                &format!("`{name}` must not appear more than once"),
            );
        }
    }
    if field("jsonrpc") != Some(&JsonValue::String("2.0".to_string())) {
        jsonrpc_fixture_error(
            manifest_path,
            line_number,
            index,
            &format!("$[{index}].jsonrpc"),
            "`jsonrpc` must be the string `2.0`",
        );
    }
    if !matches!(field("method"), Some(JsonValue::String(_))) {
        jsonrpc_fixture_error(
            manifest_path,
            line_number,
            index,
            &format!("$[{index}].method"),
            "`method` must be a string",
        );
    }
    if let Some(id) = field("id")
        && !matches!(
            id,
            JsonValue::Null | JsonValue::String(_) | JsonValue::Number(_) | JsonValue::Decimal(_)
        )
    {
        jsonrpc_fixture_error(
            manifest_path,
            line_number,
            index,
            &format!("$[{index}].id"),
            "`id` must be a string, number, or null",
        );
    }
    if let Some(params) = field("params")
        && !matches!(
            params,
            JsonValue::Null | JsonValue::Array(_) | JsonValue::Object(_)
        )
    {
        jsonrpc_fixture_error(
            manifest_path,
            line_number,
            index,
            &format!("$[{index}].params"),
            "`params` must be an object, array, or null",
        );
    }
}

pub(super) fn jsonrpc_parse_error_message_context(
    text: &str,
    error_offset: usize,
) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut offset = skip_json_ws(bytes, 0);
    if bytes.get(offset) != Some(&b'[') {
        return None;
    }
    offset += 1;
    let mut index = 0;
    loop {
        offset = skip_json_ws(bytes, offset);
        match bytes.get(offset) {
            Some(b']') => return None,
            Some(_) if error_offset <= offset => return Some(index),
            Some(_) => {}
            None => return Some(index),
        }
        match skip_json_value(bytes, offset, error_offset) {
            Some(next) => offset = skip_json_ws(bytes, next),
            None => return Some(index),
        }
        match bytes.get(offset) {
            Some(b',') => {
                offset += 1;
                index += 1;
            }
            Some(b']') => return None,
            Some(_) | None => return Some(index),
        }
    }
}

pub(super) fn skip_json_ws(bytes: &[u8], mut offset: usize) -> usize {
    while matches!(bytes.get(offset), Some(b' ' | b'\n' | b'\r' | b'\t')) {
        offset += 1;
    }
    offset
}

pub(super) fn skip_json_value(bytes: &[u8], offset: usize, stop: usize) -> Option<usize> {
    let offset = skip_json_ws(bytes, offset);
    match bytes.get(offset)? {
        b'"' => skip_json_string(bytes, offset, stop),
        b'{' => skip_json_container(bytes, offset, stop, b'{', b'}'),
        b'[' => skip_json_container(bytes, offset, stop, b'[', b']'),
        b'-' | b'0'..=b'9' => skip_json_number(bytes, offset),
        b'n' if bytes.get(offset..offset + 4) == Some(b"null") => Some(offset + 4),
        b't' if bytes.get(offset..offset + 4) == Some(b"true") => Some(offset + 4),
        b'f' if bytes.get(offset..offset + 5) == Some(b"false") => Some(offset + 5),
        _ => None,
    }
}

pub(super) fn skip_json_container(
    bytes: &[u8],
    mut offset: usize,
    stop: usize,
    open: u8,
    close: u8,
) -> Option<usize> {
    if bytes.get(offset) != Some(&open) {
        return None;
    }
    let mut depth = 0usize;
    while offset < bytes.len() {
        if offset >= stop {
            return None;
        }
        match bytes[offset] {
            byte if byte == open => {
                depth += 1;
                offset += 1;
            }
            byte if byte == close => {
                depth -= 1;
                offset += 1;
                if depth == 0 {
                    return Some(offset);
                }
            }
            b'"' => offset = skip_json_string(bytes, offset, stop)?,
            _ => offset += 1,
        }
    }
    None
}

pub(super) fn skip_json_string(bytes: &[u8], mut offset: usize, stop: usize) -> Option<usize> {
    if bytes.get(offset) != Some(&b'"') {
        return None;
    }
    offset += 1;
    while offset < bytes.len() {
        if offset >= stop {
            return None;
        }
        match bytes[offset] {
            b'"' => return Some(offset + 1),
            b'\\' => offset += 2,
            _ => offset += 1,
        }
    }
    None
}

pub(super) fn skip_json_number(bytes: &[u8], mut offset: usize) -> Option<usize> {
    if bytes.get(offset) == Some(&b'-') {
        offset += 1;
    }
    match bytes.get(offset)? {
        b'0' => offset += 1,
        b'1'..=b'9' => {
            offset += 1;
            while matches!(bytes.get(offset), Some(b'0'..=b'9')) {
                offset += 1;
            }
        }
        _ => return None,
    }
    if bytes.get(offset) == Some(&b'.') {
        offset += 1;
        if !matches!(bytes.get(offset), Some(b'0'..=b'9')) {
            return None;
        }
        while matches!(bytes.get(offset), Some(b'0'..=b'9')) {
            offset += 1;
        }
    }
    if matches!(bytes.get(offset), Some(b'e' | b'E')) {
        offset += 1;
        if matches!(bytes.get(offset), Some(b'+' | b'-')) {
            offset += 1;
        }
        if !matches!(bytes.get(offset), Some(b'0'..=b'9')) {
            return None;
        }
        while matches!(bytes.get(offset), Some(b'0'..=b'9')) {
            offset += 1;
        }
    }
    Some(offset)
}
