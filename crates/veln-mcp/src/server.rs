use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use serde_json::{Value, json};

use crate::check_project;
use crate::definition;
use crate::language_resources::{LanguageResources, ResourceCapacityError};
use crate::language_tools;
use crate::outcome::ToolOutcome;
use crate::references;
use crate::schema;
use crate::workspace::{Selection, WorkspaceBase};

const PROTOCOL_VERSION: &str = "2025-06-18";

pub(crate) fn run(base: PathBuf, reader: impl BufRead, mut writer: impl Write) -> io::Result<()> {
    let base = WorkspaceBase::open(base)?;
    let mut server = Server {
        selection: Selection::discover(base.path())?,
        base,
        initialized: false,
        language_resources: LanguageResources::checked().map_err(io::Error::other)?,
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
    base: WorkspaceBase,
    selection: Selection,
    initialized: bool,
    language_resources: LanguageResources,
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
        let request = match Request::parse(&request, raw_id) {
            Ok(Some(request)) => request,
            Ok(None) => return None,
            Err(response) => return Some(response),
        };
        let result = self.dispatch(request.method, request.params);
        Some(match result {
            Ok(result) => JsonRpcResponse::result(request.response_id, result),
            Err(RequestError::InvalidParams(message)) => {
                protocol_error(request.response_id, -32602, message)
            }
            Err(RequestError::MethodNotFound) => {
                protocol_error(request.response_id, -32601, "Method not found")
            }
            Err(RequestError::ResourceNotFound) => JsonRpcResponse::error_with_data(
                request.response_id,
                -32002,
                "Resource not found",
                json!({"code": "resource_not_found"}),
            ),
        })
    }

    fn dispatch(&mut self, method: &str, params: Option<&Value>) -> Result<Value, RequestError> {
        match method {
            "initialize" => self.initialize(params).map_err(RequestError::InvalidParams),
            _ if !self.initialized => Err(RequestError::InvalidParams("Server not initialized")),
            "ping" => request_metadata_params(params)
                .map(|()| json!({}))
                .map_err(RequestError::InvalidParams),
            "tools/list" => list_tools_params(params)
                .map(|()| json!({"tools": schema::declarations().clone()}))
                .map_err(RequestError::InvalidParams),
            "tools/call" => self.call_tool(params).map_err(RequestError::InvalidParams),
            "resources/list" => self
                .list_resources(params)
                .map_err(RequestError::InvalidParams),
            "resources/read" => self.read_resource(params),
            _ => Err(RequestError::MethodNotFound),
        }
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
            "capabilities": {
                "tools": {"listChanged": false},
                "resources": {"listChanged": false, "subscribe": false}
            },
            "serverInfo": {"name": "veln", "version": env!("CARGO_PKG_VERSION")}
        }))
    }

    fn list_resources(&self, params: Option<&Value>) -> Result<Value, &'static str> {
        list_resources_params(params)?;
        Ok(self.language_resources.list_result())
    }

    fn read_resource(&self, params: Option<&Value>) -> Result<Value, RequestError> {
        let uri = read_resource_uri(params)?;
        self.language_resources
            .read_result(uri)
            .ok_or(RequestError::ResourceNotFound)
    }

    fn call_tool(&mut self, params: Option<&Value>) -> Result<Value, &'static str> {
        let base = self.base.clone();
        self.call_tool_with_refresh(params, |selection| selection.refresh(base.path()))
    }

    fn call_tool_with_refresh(
        &mut self,
        params: Option<&Value>,
        refresh: impl FnOnce(&mut Selection) -> io::Result<()>,
    ) -> Result<Value, &'static str> {
        let call = ToolCall::parse(params)?;
        let empty_arguments = json!({});
        let arguments = call.arguments.unwrap_or(&empty_arguments);
        if !call.tool.accepts_input(arguments) {
            return Err("Tool input does not match its schema");
        }
        Ok(match call.name {
            "workspace_projects" => render_tool_outcome(
                "workspace_projects",
                ToolOutcome::Success(self.selection_result()),
            ),
            "refresh_workspace" => self.refresh_workspace_tool(refresh),
            "check_project" => self.check_project_tool(arguments),
            "definition" => self.definition_tool(arguments),
            "references" => self.references_tool(arguments),
            "search_docs" => self.search_docs_tool(arguments),
            "read_doc" => self.read_doc_tool(arguments),
            _ => unreachable!("tool name was checked against declarations"),
        })
    }

    fn search_docs_tool(&self, arguments: &Value) -> Value {
        render_tool_outcome(
            "search_docs",
            language_tools::search_docs(&self.language_resources, arguments),
        )
    }

    fn read_doc_tool(&self, arguments: &Value) -> Value {
        render_tool_outcome(
            "read_doc",
            language_tools::read_doc(&self.language_resources, arguments),
        )
    }

    fn references_tool(&mut self, arguments: &Value) -> Value {
        render_tool_outcome(
            "references",
            references::references(
                &self.base,
                &self.selection,
                &mut self.language_resources,
                arguments,
            ),
        )
    }

    fn definition_tool(&mut self, arguments: &Value) -> Value {
        render_tool_outcome(
            "definition",
            definition::definition(
                &self.base,
                &self.selection,
                &mut self.language_resources,
                arguments,
            ),
        )
    }

    fn check_project_tool(&mut self, arguments: &Value) -> Value {
        render_tool_outcome(
            "check_project",
            check_project::check_project(
                &self.base,
                &self.selection,
                &mut self.language_resources,
                arguments,
            ),
        )
    }

    fn refresh_workspace_tool(
        &mut self,
        refresh: impl FnOnce(&mut Selection) -> io::Result<()>,
    ) -> Value {
        let outcome = match refresh(&mut self.selection) {
            Ok(()) => ToolOutcome::Success(self.selection_result()),
            Err(_) => ToolOutcome::DomainFailure {
                code: "generation_failed",
                message: "workspace project discovery failed",
                details: json!({}),
            },
        };
        render_tool_outcome("refresh_workspace", outcome)
    }

    fn selection_result(&self) -> Value {
        json!({
            "generation": self.selection.generation(),
            "roots": self.selection.roots(),
        })
    }
}

pub(crate) fn resource_capacity_failure() -> ToolOutcome {
    ToolOutcome::DomainFailure {
        code: "resource_capacity",
        message: "dependency source resource capacity exceeded",
        details: json!({}),
    }
}

impl From<ResourceCapacityError> for ToolOutcome {
    fn from(_: ResourceCapacityError) -> Self {
        resource_capacity_failure()
    }
}

struct Request<'a> {
    response_id: ResponseId,
    method: &'a str,
    params: Option<&'a Value>,
}

impl<'a> Request<'a> {
    fn parse(
        request: &'a Value,
        raw_id: Option<ResponseId>,
    ) -> Result<Option<Self>, JsonRpcResponse> {
        let Some(object) = request.as_object() else {
            return Err(protocol_error(Value::Null, -32600, "Invalid Request"));
        };
        let id = object.get("id").cloned();
        if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
            || !id.as_ref().is_none_or(valid_request_id)
        {
            return Err(protocol_error(Value::Null, -32600, "Invalid Request"));
        }
        let Some(method) = object.get("method").and_then(Value::as_str) else {
            return Err(protocol_error(Value::Null, -32600, "Invalid Request"));
        };
        let params = object.get("params");
        let Some(response_id) = raw_id.or_else(|| id.map(ResponseId::from_value)) else {
            return Ok(None);
        };
        Ok(Some(Self {
            response_id,
            method,
            params,
        }))
    }
}

enum RequestError {
    InvalidParams(&'static str),
    MethodNotFound,
    ResourceNotFound,
}

struct ToolCall<'a> {
    name: &'a str,
    arguments: Option<&'a Value>,
    tool: schema::ToolSchema,
}

impl<'a> ToolCall<'a> {
    fn parse(params: Option<&'a Value>) -> Result<Self, &'static str> {
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
        Ok(Self {
            name,
            arguments: params.get("arguments"),
            tool,
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

fn list_resources_params(params: Option<&Value>) -> Result<(), &'static str> {
    match params {
        None => Ok(()),
        Some(Value::Object(object))
            if object.keys().all(|key| key == "_meta")
                && metadata_is_valid(object.get("_meta")) =>
        {
            Ok(())
        }
        _ => Err("Invalid resource list params"),
    }
}

fn read_resource_uri(params: Option<&Value>) -> Result<&str, RequestError> {
    let params = params
        .and_then(Value::as_object)
        .ok_or(RequestError::InvalidParams("Invalid resource read params"))?;
    if params.keys().any(|key| key != "uri" && key != "_meta")
        || !metadata_is_valid(params.get("_meta"))
    {
        return Err(RequestError::InvalidParams("Invalid resource read params"));
    }
    params
        .get("uri")
        .and_then(Value::as_str)
        .ok_or(RequestError::InvalidParams("Invalid resource read params"))
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

fn render_tool_outcome(tool_name: &str, outcome: ToolOutcome) -> Value {
    let tool = schema::tool(tool_name).expect("tool outcome belongs to a declared tool");
    let (structured, is_error) = match outcome {
        ToolOutcome::Success(result) => (result, false),
        ToolOutcome::DomainFailure {
            code,
            message,
            details,
        } => (
            json!({"code": code, "message": message, "details": details}),
            true,
        ),
    };
    assert!(
        tool.accepts_result(&structured),
        "{tool_name} outcome must match the advertised schema: {structured}"
    );
    json!({
        "content": [{"type": "text", "text": structured.to_string()}],
        "structuredContent": structured,
        "isError": is_error,
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

    fn error_with_data(id: ResponseId, code: i64, message: &str, data: Value) -> Self {
        Self::Error {
            id,
            error: json!({"code": code, "message": message, "data": data}),
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
mod tests;
