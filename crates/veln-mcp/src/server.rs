use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use serde_json::{Value, json};

use crate::schema;
use crate::workspace::Selection;

const PROTOCOL_VERSION: &str = "2025-06-18";

pub(crate) fn run(base: PathBuf, reader: impl BufRead, mut writer: impl Write) -> io::Result<()> {
    let mut server = Server {
        selection: Selection::discover(&base)?,
        base,
        initialized: false,
    };
    for line in reader.lines() {
        if let Some(response) = server.handle_line(&line?) {
            response.write_json(&mut writer)?;
            writer.write_all(b"\n")?;
            writer.flush()?;
        }
    }
    Ok(())
}

struct Server {
    base: PathBuf,
    selection: Selection,
    initialized: bool,
}

impl Server {
    fn handle_line(&mut self, line: &str) -> Option<JsonRpcResponse> {
        let request = match serde_json::from_str::<Value>(line) {
            Ok(request) => request,
            Err(_) => return Some(protocol_error(Value::Null, -32700, "Parse error")),
        };
        self.handle_request_with_id(request, request_id_from_line(line))
    }

    #[cfg(test)]
    fn handle_request(&mut self, request: Value) -> Option<Value> {
        self.handle_request_with_id(request, None)
            .map(JsonRpcResponse::into_value)
    }

    fn handle_request_with_id(
        &mut self,
        request: Value,
        raw_id: Option<ResponseId>,
    ) -> Option<JsonRpcResponse> {
        let Some(object) = request.as_object() else {
            return Some(protocol_error(Value::Null, -32600, "Invalid Request"));
        };
        let id = object.get("id").cloned();
        if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
            || !id.as_ref().is_none_or(valid_request_id)
        {
            return id.map(|_| protocol_error(Value::Null, -32600, "Invalid Request"));
        }
        let Some(method) = object.get("method").and_then(Value::as_str) else {
            return id.map(|_| protocol_error(Value::Null, -32600, "Invalid Request"));
        };
        let params = object.get("params");

        let response_id = raw_id.or_else(|| id.map(ResponseId::from_value))?;

        let result = match method {
            "initialize" => self.initialize(params),
            _ if !self.initialized => Err("Server not initialized"),
            "ping" => request_metadata_params(params).map(|()| json!({})),
            "tools/list" => {
                list_tools_params(params).map(|()| json!({"tools": schema::declarations().clone()}))
            }
            "tools/call" => self.call_tool(params),
            _ => return Some(protocol_error(response_id, -32601, "Method not found")),
        };
        Some(match result {
            Ok(result) => JsonRpcResponse::result(response_id, result),
            Err(message) => protocol_error(response_id, -32602, message),
        })
    }

    fn initialize(&mut self, params: Option<&Value>) -> Result<Value, &'static str> {
        let Some(params) = params.and_then(Value::as_object) else {
            return Err("Invalid initialize params");
        };
        let protocol_version = params.get("protocolVersion").and_then(Value::as_str);
        let capabilities = params.get("capabilities");
        let client_info = params.get("clientInfo");
        if protocol_version.is_none()
            || !capabilities.is_some_and(Value::is_object)
            || !client_info.is_some_and(valid_implementation)
            || !metadata_is_valid(params.get("_meta"))
        {
            return Err("Invalid initialize params");
        }
        if self.initialized {
            return Err("Server already initialized");
        }
        self.initialized = true;
        Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {"tools": {"listChanged": false}},
            "serverInfo": {"name": "veln", "version": env!("CARGO_PKG_VERSION")}
        }))
    }

    fn call_tool(&mut self, params: Option<&Value>) -> Result<Value, &'static str> {
        let base = self.base.clone();
        self.call_tool_with_refresh(params, |selection| selection.refresh(&base))
    }

    fn call_tool_with_refresh(
        &mut self,
        params: Option<&Value>,
        refresh: impl FnOnce(&mut Selection) -> io::Result<()>,
    ) -> Result<Value, &'static str> {
        let params = params
            .and_then(Value::as_object)
            .ok_or("Invalid tool call params")?;
        if params
            .keys()
            .any(|key| key != "name" && key != "arguments" && key != "_meta")
            || !metadata_is_valid(params.get("_meta"))
        {
            return Err("Invalid tool call params");
        }
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or("Invalid tool call params")?;
        let tool = schema::tool(name).ok_or("Unknown tool")?;
        let empty_arguments = json!({});
        let arguments = params.get("arguments").unwrap_or(&empty_arguments);
        if !tool.accepts_input(arguments) {
            return Err("Tool input does not match its schema");
        }

        let result = match name {
            "workspace_projects" => return Ok(successful_tool_result(self.selection_result())),
            "refresh_workspace" => match refresh(&mut self.selection) {
                Ok(()) => return Ok(successful_tool_result(self.selection_result())),
                Err(_) => domain_failure(
                    "generation_failed",
                    "workspace project discovery failed",
                    json!({}),
                ),
            },
            _ => unreachable!("tool name was checked against declarations"),
        };
        Ok(result)
    }

    fn selection_result(&self) -> Value {
        json!({
            "generation": self.selection.generation(),
            "roots": self.selection.roots(),
        })
    }
}

fn valid_request_id(value: &Value) -> bool {
    value.is_string() || value.is_number()
}

fn valid_implementation(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    object.get("name").and_then(Value::as_str).is_some()
        && object.get("version").and_then(Value::as_str).is_some()
}

fn request_metadata_params(params: Option<&Value>) -> Result<(), &'static str> {
    match params {
        None => Ok(()),
        Some(Value::Object(object))
            if object.keys().all(|key| key == "_meta")
                && metadata_is_valid(object.get("_meta")) =>
        {
            Ok(())
        }
        _ => Err("Invalid params"),
    }
}

fn list_tools_params(params: Option<&Value>) -> Result<(), &'static str> {
    match params {
        None => Ok(()),
        Some(Value::Object(object))
            if object.keys().all(|key| key == "cursor" || key == "_meta")
                && object.get("cursor").is_none_or(Value::is_string)
                && metadata_is_valid(object.get("_meta")) =>
        {
            Ok(())
        }
        _ => Err("Invalid params"),
    }
}

fn metadata_is_valid(value: Option<&Value>) -> bool {
    let Some(value) = value else {
        return true;
    };
    let Some(object) = value.as_object() else {
        return false;
    };
    object
        .get("progressToken")
        .is_none_or(|token| token.is_string() || token.is_number())
}

fn successful_tool_result(structured: Value) -> Value {
    json!({
        "content": [{"type": "text", "text": structured.to_string()}],
        "structuredContent": structured,
        "isError": false,
    })
}

fn domain_failure(code: &str, message: &str, details: Value) -> Value {
    let structured = json!({"code": code, "message": message, "details": details});
    json!({
        "content": [{"type": "text", "text": structured.to_string()}],
        "structuredContent": structured,
        "isError": true,
    })
}

fn protocol_error(id: impl Into<ResponseId>, code: i64, message: &str) -> JsonRpcResponse {
    JsonRpcResponse::error(id.into(), code, message)
}

#[derive(Clone)]
enum ResponseId {
    Value(Value),
    RawNumber(String),
}

impl ResponseId {
    fn from_value(value: Value) -> Self {
        Self::Value(value)
    }

    fn write_json(&self, writer: &mut impl Write) -> io::Result<()> {
        match self {
            ResponseId::Value(value) => {
                serde_json::to_writer(writer, value).map_err(io::Error::other)
            }
            ResponseId::RawNumber(raw) => writer.write_all(raw.as_bytes()),
        }
    }

    #[cfg(test)]
    fn into_value(self) -> Value {
        match self {
            ResponseId::Value(value) => value,
            ResponseId::RawNumber(raw) => {
                serde_json::from_str(&raw).expect("raw numeric request id was parsed from JSON")
            }
        }
    }
}

impl From<Value> for ResponseId {
    fn from(value: Value) -> Self {
        Self::Value(value)
    }
}

enum JsonRpcResponse {
    Result { id: ResponseId, result: Value },
    Error { id: ResponseId, error: Value },
}

impl JsonRpcResponse {
    fn result(id: ResponseId, result: Value) -> Self {
        Self::Result { id, result }
    }

    fn error(id: ResponseId, code: i64, message: &str) -> Self {
        Self::Error {
            id,
            error: json!({"code": code, "message": message}),
        }
    }

    fn write_json(&self, writer: &mut impl Write) -> io::Result<()> {
        match self {
            JsonRpcResponse::Result { id, result } => {
                writer.write_all(b"{\"id\":")?;
                id.write_json(writer)?;
                writer.write_all(b",\"jsonrpc\":\"2.0\",\"result\":")?;
                serde_json::to_writer(&mut *writer, result).map_err(io::Error::other)?;
                writer.write_all(b"}")
            }
            JsonRpcResponse::Error { id, error } => {
                writer.write_all(b"{\"error\":")?;
                serde_json::to_writer(&mut *writer, error).map_err(io::Error::other)?;
                writer.write_all(b",\"id\":")?;
                id.write_json(writer)?;
                writer.write_all(b",\"jsonrpc\":\"2.0\"}")
            }
        }
    }

    #[cfg(test)]
    fn into_value(self) -> Value {
        match self {
            JsonRpcResponse::Result { id, result } => {
                json!({"jsonrpc": "2.0", "id": id.into_value(), "result": result})
            }
            JsonRpcResponse::Error { id, error } => {
                json!({"jsonrpc": "2.0", "id": id.into_value(), "error": error})
            }
        }
    }
}

fn request_id_from_line(line: &str) -> Option<ResponseId> {
    let id = top_level_field_lexeme(line, "id")?;
    let value = serde_json::from_str::<Value>(id).ok()?;
    if value.is_number() {
        Some(ResponseId::RawNumber(id.to_string()))
    } else {
        Some(ResponseId::Value(value))
    }
}

fn top_level_field_lexeme<'a>(text: &'a str, target: &str) -> Option<&'a str> {
    let bytes = text.as_bytes();
    let mut offset = skip_json_ws(bytes, 0);
    if bytes.get(offset) != Some(&b'{') {
        return None;
    }
    offset += 1;
    loop {
        offset = skip_json_ws(bytes, offset);
        match bytes.get(offset) {
            Some(b'}') => return None,
            Some(b'"') => {}
            _ => return None,
        }
        let key_end = skip_json_string(bytes, offset)?;
        let key = serde_json::from_str::<String>(&text[offset..key_end]).ok()?;
        offset = skip_json_ws(bytes, key_end);
        if bytes.get(offset) != Some(&b':') {
            return None;
        }
        offset = skip_json_ws(bytes, offset + 1);
        let value_start = offset;
        let value_end = skip_json_value(bytes, offset)?;
        if key == target {
            return Some(&text[value_start..value_end]);
        }
        offset = skip_json_ws(bytes, value_end);
        match bytes.get(offset) {
            Some(b',') => offset += 1,
            Some(b'}') => return None,
            _ => return None,
        }
    }
}

fn skip_json_ws(bytes: &[u8], mut offset: usize) -> usize {
    while matches!(bytes.get(offset), Some(b' ' | b'\n' | b'\r' | b'\t')) {
        offset += 1;
    }
    offset
}

fn skip_json_value(bytes: &[u8], offset: usize) -> Option<usize> {
    match bytes.get(offset)? {
        b'"' => skip_json_string(bytes, offset),
        b'{' => skip_json_container(bytes, offset, b'{', b'}'),
        b'[' => skip_json_container(bytes, offset, b'[', b']'),
        b'-' | b'0'..=b'9' => skip_json_number(bytes, offset),
        b't' => bytes
            .get(offset..offset + 4)
            .is_some_and(|slice| slice == b"true")
            .then_some(offset + 4),
        b'f' => bytes
            .get(offset..offset + 5)
            .is_some_and(|slice| slice == b"false")
            .then_some(offset + 5),
        b'n' => bytes
            .get(offset..offset + 4)
            .is_some_and(|slice| slice == b"null")
            .then_some(offset + 4),
        _ => None,
    }
}

fn skip_json_string(bytes: &[u8], mut offset: usize) -> Option<usize> {
    if bytes.get(offset) != Some(&b'"') {
        return None;
    }
    offset += 1;
    while let Some(byte) = bytes.get(offset) {
        match byte {
            b'"' => return Some(offset + 1),
            b'\\' => offset += 2,
            _ => offset += 1,
        }
    }
    None
}

fn skip_json_container(bytes: &[u8], mut offset: usize, open: u8, close: u8) -> Option<usize> {
    if bytes.get(offset) != Some(&open) {
        return None;
    }
    let mut depth = 1usize;
    offset += 1;
    while let Some(byte) = bytes.get(offset) {
        match byte {
            b'"' => offset = skip_json_string(bytes, offset)?,
            byte if *byte == open => {
                depth += 1;
                offset += 1;
            }
            byte if *byte == close => {
                depth -= 1;
                offset += 1;
                if depth == 0 {
                    return Some(offset);
                }
            }
            _ => offset += 1,
        }
    }
    None
}

fn skip_json_number(bytes: &[u8], mut offset: usize) -> Option<usize> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn lifecycle_lists_and_calls_only_implemented_tools() {
        let workspace = TempWorkspace::new("lifecycle");
        workspace.write("alpha/veln.toml", "");
        workspace.write("beta/deep/veln.toml", "");
        let input = [
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":{"progressToken":"list"}}}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"workspace_projects","arguments":{}}}),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"refresh_workspace","arguments":{}}}),
        ]
        .into_iter()
        .map(|message| message.to_string())
        .collect::<Vec<_>>()
        .join("\n");
        let mut output = Vec::new();
        run(workspace.root.clone(), input.as_bytes(), &mut output).unwrap();
        let responses = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(responses.len(), 4);
        let names = responses[1]["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(names, ["workspace_projects", "refresh_workspace"]);
        assert_eq!(
            responses[2]["result"]["structuredContent"],
            json!({
                "generation": 0,
                "roots": ["alpha", "beta/deep"]
            })
        );
        assert_eq!(responses[3]["result"]["structuredContent"]["generation"], 1);
    }

    #[test]
    fn client_root_information_does_not_change_selection() {
        let workspace = TempWorkspace::new("client-roots");
        workspace.write("nested/veln.toml", "");
        let variants = [
            json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}),
            json!({"protocolVersion":"2025-06-18","capabilities":{"roots":{"listChanged":true}},"clientInfo":{"name":"test","version":"1"},"rootUri":"file:///unrelated"}),
            json!({"protocolVersion":"2025-06-18","capabilities":{"roots":{"listChanged":false}},"clientInfo":{"name":"test","version":"1"},"roots":[{"uri":"file:///nested","name":"nested"}]}),
        ];

        for params in variants {
            let input = format!(
                "{}\n{}\n",
                json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":params}),
                json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"workspace_projects","arguments":{}}})
            );
            let mut output = Vec::new();
            run(workspace.root.clone(), input.as_bytes(), &mut output).unwrap();
            let response = String::from_utf8(output).unwrap();
            let selection =
                serde_json::from_str::<Value>(response.lines().nth(1).unwrap()).unwrap();
            assert_eq!(
                selection["result"]["structuredContent"]["roots"],
                json!(["nested"])
            );
        }
    }

    #[test]
    fn invalid_tool_inputs_are_protocol_invalid_params() {
        let workspace = TempWorkspace::new("invalid-input");
        let selection = Selection::discover(&workspace.root).unwrap();
        let mut server = Server {
            base: workspace.root.clone(),
            selection,
            initialized: true,
        };
        let requests = [
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"workspace_projects","arguments":{"unknown":true}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"refresh_workspace","arguments":[]}}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"refresh_workspace","arguments":null}}),
        ];
        for request in requests {
            let response = server.handle_request(request).unwrap();
            assert_eq!(response["error"]["code"], -32602);
            assert!(response.get("result").is_none());
        }
    }

    #[test]
    fn initialize_requires_the_declared_wire_shape() {
        let workspace = TempWorkspace::new("initialize-wire-shape");
        let selection = Selection::discover(&workspace.root).unwrap();
        let mut server = Server {
            base: workspace.root.clone(),
            selection,
            initialized: false,
        };
        let valid = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1"},
                "_meta": {"progressToken": "startup"}
            }
        });
        let invalid_requests = [
            json!({"jsonrpc":"2.0","id":2,"method":"initialize","params":{"capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
            json!({"jsonrpc":"2.0","id":3,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"1"}}}),
            json!({"jsonrpc":"2.0","id":4,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test"}}}),
            json!({"jsonrpc":"2.0","id":5,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"},"_meta":{"progressToken":null}}}),
        ];
        for request in invalid_requests {
            let response = server.handle_request(request).unwrap();
            assert_eq!(response["error"]["code"], -32602);
            assert!(response.get("result").is_none());
        }

        assert!(
            server
                .handle_request(valid.clone())
                .unwrap()
                .get("result")
                .is_some()
        );
        let repeated = server.handle_request(valid).unwrap();
        assert_eq!(repeated["error"]["code"], -32602);
        assert_eq!(repeated["error"]["message"], "Server already initialized");
        assert!(repeated.get("result").is_none());
    }

    #[test]
    fn lifecycle_rejects_operations_before_initialize() {
        let workspace = TempWorkspace::new("lifecycle-before-initialize");
        let selection = Selection::discover(&workspace.root).unwrap();
        let mut server = Server {
            base: workspace.root.clone(),
            selection,
            initialized: false,
        };

        let response = server
            .handle_request(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
            .unwrap();
        assert_eq!(response["error"]["code"], -32602);
        assert_eq!(response["error"]["message"], "Server not initialized");
        assert!(response.get("result").is_none());
    }

    #[test]
    fn request_ids_accept_strings_and_numbers_but_reject_null() {
        let workspace = TempWorkspace::new("request-ids");
        let selection = Selection::discover(&workspace.root).unwrap();
        let mut server = Server {
            base: workspace.root.clone(),
            selection,
            initialized: true,
        };

        let null_response = server
            .handle_request(json!({"jsonrpc":"2.0","id":null,"method":"ping"}))
            .unwrap();
        assert_eq!(null_response["id"], Value::Null);
        assert_eq!(null_response["error"]["code"], -32600);

        let fractional_response = server
            .handle_request(json!({"jsonrpc":"2.0","id":1.5,"method":"ping"}))
            .unwrap();
        assert_eq!(fractional_response["id"], json!(1.5));
        assert_eq!(fractional_response["result"], json!({}));

        let response = server
            .handle_request(json!({"jsonrpc":"2.0","id":"ok","method":"ping","params":{"_meta":{"progressToken":7.5}}}))
            .unwrap();
        assert_eq!(response["result"], json!({}));
    }

    #[test]
    fn stdio_preserves_large_adjacent_numeric_request_ids() {
        let workspace = TempWorkspace::new("large-request-ids");
        let input = [
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}"#,
            r#"{"jsonrpc":"2.0","id":18446744073709551616,"method":"ping"}"#,
            r#"{"jsonrpc":"2.0","id":18446744073709551617,"method":"ping"}"#,
        ]
        .join("\n");
        let mut output = Vec::new();
        run(workspace.root.clone(), input.as_bytes(), &mut output).unwrap();
        let response = String::from_utf8(output).unwrap();
        let lines = response.lines().collect::<Vec<_>>();

        assert_eq!(lines.len(), 3);
        assert!(
            lines[1].contains(r#""id":18446744073709551616"#),
            "{response}"
        );
        assert!(
            lines[2].contains(r#""id":18446744073709551617"#),
            "{response}"
        );
    }

    #[test]
    fn refresh_domain_failure_preserves_the_observable_selection() {
        let workspace = TempWorkspace::new("refresh-domain-failure");
        workspace.write("alpha/veln.toml", "");
        let selection = Selection::discover(&workspace.root).unwrap();
        let mut server = Server {
            base: workspace.root.clone(),
            selection,
            initialized: true,
        };
        let refresh_params = json!({"name":"refresh_workspace","arguments":{}});

        let result = server
            .call_tool_with_refresh(Some(&refresh_params), |selection| {
                selection.refresh_with(|| Err(io::Error::other("injected discovery failure")))
            })
            .unwrap();
        assert_eq!(result["isError"], true);
        assert_eq!(
            result["structuredContent"],
            json!({
                "code": "generation_failed",
                "message": "workspace project discovery failed",
                "details": {}
            })
        );

        let list_params = json!({"name":"workspace_projects","arguments":{}});
        let following = server.call_tool(Some(&list_params)).unwrap();
        assert_eq!(
            following["structuredContent"],
            json!({"generation": 0, "roots": ["alpha"]})
        );
    }

    struct TempWorkspace {
        root: PathBuf,
    }

    impl TempWorkspace {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "veln-mcp-server-{name}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.root.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }
    }

    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
