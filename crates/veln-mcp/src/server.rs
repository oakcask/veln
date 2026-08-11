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
    };
    for line in reader.lines() {
        if let Some(response) = server.handle_line(&line?) {
            serde_json::to_writer(&mut writer, &response)?;
            writer.write_all(b"\n")?;
            writer.flush()?;
        }
    }
    Ok(())
}

struct Server {
    base: PathBuf,
    selection: Selection,
}

impl Server {
    fn handle_line(&mut self, line: &str) -> Option<Value> {
        let request = match serde_json::from_str::<Value>(line) {
            Ok(request) => request,
            Err(_) => return Some(protocol_error(Value::Null, -32700, "Parse error")),
        };
        self.handle_request(request)
    }

    fn handle_request(&mut self, request: Value) -> Option<Value> {
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

        let response_id = id?;

        let result = match method {
            "initialize" => self.initialize(params),
            "ping" => request_metadata_params(params).map(|()| json!({})),
            "tools/list" => {
                list_tools_params(params).map(|()| json!({"tools": schema::declarations().clone()}))
            }
            "tools/call" => self.call_tool(params),
            _ => return Some(protocol_error(response_id, -32601, "Method not found")),
        };
        Some(match result {
            Ok(result) => json!({"jsonrpc": "2.0", "id": response_id, "result": result}),
            Err(message) => protocol_error(response_id, -32602, message),
        })
    }

    fn initialize(&self, params: Option<&Value>) -> Result<Value, &'static str> {
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
    value.is_string() || value.as_i64().is_some() || value.as_u64().is_some()
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
    object.get("progressToken").is_none_or(|token| {
        token.is_string() || token.as_i64().is_some() || token.as_u64().is_some()
    })
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

fn protocol_error(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message},
    })
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
        assert!(
            server
                .handle_request(valid)
                .unwrap()
                .get("result")
                .is_some()
        );

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
    }

    #[test]
    fn request_ids_reject_null_and_fractional_numbers() {
        let workspace = TempWorkspace::new("request-ids");
        let selection = Selection::discover(&workspace.root).unwrap();
        let mut server = Server {
            base: workspace.root.clone(),
            selection,
        };
        let requests = [
            json!({"jsonrpc":"2.0","id":null,"method":"ping"}),
            json!({"jsonrpc":"2.0","id":1.5,"method":"ping"}),
        ];
        for request in requests {
            let response = server.handle_request(request).unwrap();
            assert_eq!(response["id"], Value::Null);
            assert_eq!(response["error"]["code"], -32600);
        }
        let response = server
            .handle_request(json!({"jsonrpc":"2.0","id":"ok","method":"ping","params":{"_meta":{"progressToken":7}}}))
            .unwrap();
        assert_eq!(response["result"], json!({}));
    }

    #[test]
    fn refresh_domain_failure_preserves_the_observable_selection() {
        let workspace = TempWorkspace::new("refresh-domain-failure");
        workspace.write("alpha/veln.toml", "");
        let selection = Selection::discover(&workspace.root).unwrap();
        let mut server = Server {
            base: workspace.root.clone(),
            selection,
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
