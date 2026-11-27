use super::*;
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, ChildStdin};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(30);
const MARKDOWN: &str = "text/markdown; charset=utf-8";

// Integration contract: the CLI publishes the embedded std catalog over stdio,
// and every selected module/declaration is readable through its published link.
// Content and full signatures are checked by the bundle's publication tests.
#[test]
fn standard_library_package_documentation_resources_follow_published_links() {
    let project = TestProject::new(
        "package-documentation-resources".into(),
        &ToolSetup::default(),
    );
    let mut session = Session::new(&project.root);
    session.request("initialize", r#"{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"spec","version":"1"}}"#);
    let (resources, base, index_text) = read_standard_library_index(&mut session);
    assert_templates(&mut session);
    let modules = read_documented_modules(&mut session, &resources, &base, &index_text);
    assert_documented_declarations(&mut session, &resources, &base, &modules);
    assert_unknown_documentation_digest(&mut session, &base);
    session.finish();
}

fn read_standard_library_index(session: &mut Session) -> (Vec<JsonValue>, String, String) {
    let list = session.request("resources/list", "{}");
    let result = field(&list, "result");
    assert!(result.object_field("nextCursor").is_none());
    let resources = field(result, "resources").as_array().unwrap().to_vec();
    let indices: Vec<_> = resources
        .iter()
        .filter(|resource| {
            resource.object_field("name").and_then(JsonValue::as_str)
                == Some("std-documentation-index")
        })
        .collect();
    assert_eq!(
        indices.len(),
        1,
        "expected exactly one std documentation index"
    );
    let index = indices[0];
    assert_eq!(string(index, "title"), "Veln package documentation: std");
    assert_eq!(string(index, "mimeType"), MARKDOWN);
    assert!(index.object_field("description").is_none());
    let index_uri = string(index, "uri");
    let base = index_uri.strip_suffix("/index").unwrap().to_owned();
    assert!(base.starts_with("veln-doc:///package/std/snapshot/"));
    let index_text = session.read(index_uri);
    assert!(index_text.starts_with("# Package Documentation: std\n"));
    assert!(index_text.contains("- Package identity: std"));
    (resources, base, index_text)
}

fn read_documented_modules(
    session: &mut Session,
    resources: &[JsonValue],
    base: &str,
    index_text: &str,
) -> BTreeMap<&'static str, String> {
    let mut modules = BTreeMap::new();
    for name in ["prelude", "net", "observe", "trace"] {
        let uri = linked_uri(index_text, name);
        assert_hidden_resource(resources, &uri, base, "module");
        let text = session.read(&uri);
        assert!(text.starts_with(&format!("# Module {name}\n")));
        assert!(text.contains(&format!("- Source path: {name}.veln")));
        modules.insert(name, text);
    }
    modules
}

fn assert_documented_declarations(
    session: &mut Session,
    resources: &[JsonValue],
    base: &str,
    modules: &BTreeMap<&str, String>,
) {
    for (module, label, heading) in declaration_reads() {
        let uri = linked_uri(&modules[module], label);
        assert_hidden_resource(resources, &uri, base, "declaration");
        let text = session.read(&uri);
        assert!(
            text.starts_with(&format!("{heading}\n")),
            "{module}::{label}"
        );
        assert_private_details_absent(&text);
    }
    for (module, labels) in [
        (
            "net",
            &[
                "alias Listener",
                "alias Stream",
                "alias ByteCount",
                "type ReadOutcome",
                "type WriteOutcome",
            ][..],
        ),
        (
            "observe",
            &[
                "handler buffered_json_lines",
                "function buffered_start_export_span",
                "function shutdown_buffered",
            ][..],
        ),
    ] {
        for label in labels {
            let uri = linked_uri(&modules[module], label);
            assert_hidden_resource(resources, &uri, base, "declaration");
            session.read(&uri);
        }
    }
    for text in modules.values() {
        assert_private_details_absent(text);
    }
}

fn assert_unknown_documentation_digest(session: &mut Session, base: &str) {
    // Change only the documentation digest, keeping the discovered snapshot.
    let (snapshot, digest) = base.split_once("/documentation/").unwrap();
    let wrong_digest = if digest == "0".repeat(64) { "1" } else { "0" }.repeat(64);
    let uri = format!("{snapshot}/documentation/{wrong_digest}/index");
    let response = session.request(
        "resources/read",
        &format!(r#"{{"uri":{}}}"#, json_string(&uri)),
    );
    let error = field(&response, "error");
    assert_eq!(field(error, "code").as_i64(), Some(-32002));
    assert_eq!(string(field(error, "data"), "code"), "resource_not_found");
    assert!(response.object_field("result").is_none());
}

fn assert_templates(session: &mut Session) {
    let response = session.request(
        "resources/templates/list",
        r#"{"_meta":{"progressToken":"templates"}}"#,
    );
    let templates = field(field(&response, "result"), "resourceTemplates")
        .as_array()
        .unwrap();
    assert_eq!(templates.len(), 2);
    for kind in ["module", "declaration"] {
        let name = format!("package-documentation-{kind}");
        let matches: Vec<_> = templates
            .iter()
            .filter(|template| string(template, "name") == name)
            .collect();
        assert_eq!(matches.len(), 1, "{name}");
        assert_eq!(
            string(matches[0], "uriTemplate"),
            format!(
                "veln-doc:///package/{{package}}/snapshot/{{snapshot_digest}}/documentation/{{documentation_digest}}/{kind}/{{{kind}_id}}"
            )
        );
        assert_eq!(string(matches[0], "mimeType"), MARKDOWN);
    }
}

fn assert_hidden_resource(resources: &[JsonValue], uri: &str, base: &str, kind: &str) {
    assert!(
        uri.starts_with(&format!("{base}/{kind}/")),
        "foreign resource link: {uri}"
    );
    assert!(
        !resources
            .iter()
            .any(|resource| string(resource, "uri") == uri),
        "module and declaration resources must not be eagerly listed: {uri}"
    );
}

fn linked_uri(text: &str, label: &str) -> String {
    let prefix = format!("- [{label}](");
    let matches: Vec<_> = text
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix))
        .collect();
    assert_eq!(matches.len(), 1, "expected exactly one link for {label}");
    matches[0]
        .split_once(')')
        .expect("Markdown link should close")
        .0
        .to_owned()
}

fn assert_private_details_absent(text: &str) {
    for private in [
        "prelude_builtin::net_system_",
        "net_system_resolve",
        "NoopSpanRequest",
        "NoopSpanFinish",
        "NoopSpan",
    ] {
        assert!(
            !text.contains(private),
            "published private detail: {private}"
        );
    }
}

fn field<'a>(value: &'a JsonValue, name: &str) -> &'a JsonValue {
    value
        .object_field(name)
        .unwrap_or_else(|| panic!("missing {name} in {value:?}"))
}

fn string<'a>(value: &'a JsonValue, name: &str) -> &'a str {
    field(value, name).as_str().unwrap()
}

fn json_string(value: &str) -> String {
    JsonValue::String(value.to_owned()).to_compact_string()
}

// Each read has a deadline, and unwinding kills/reaps the child. A missing
// response or unexpected stdout therefore fails locally instead of hanging CI.
struct Session {
    child: Child,
    input: Option<ChildStdin>,
    output: Receiver<std::io::Result<String>>,
    reader: Option<std::thread::JoinHandle<()>>,
    next_id: i64,
}

impl Session {
    fn new(root: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_veln"))
            .arg("mcp")
            .current_dir(root)
            .env("VELN_CACHE_DIR", root.join(".veln-harness-cache"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("veln mcp should spawn");
        let input = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (sender, output) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            input,
            output,
            reader: Some(reader),
            next_id: 0,
        }
    }

    fn request(&mut self, method: &str, params: &str) -> JsonValue {
        self.next_id += 1;
        let input = self.input.as_mut().unwrap();
        writeln!(
            input,
            r#"{{"jsonrpc":"2.0","id":{},"method":{},"params":{params}}}"#,
            self.next_id,
            json_string(method)
        )
        .unwrap();
        input.flush().unwrap();
        let line = self
            .output
            .recv_timeout(TIMEOUT)
            .expect("MCP response should arrive before the deadline")
            .expect("MCP stdout should be readable");
        let response = parse_json(&line).expect("MCP stdout should contain a JSON response");
        assert_eq!(string(&response, "jsonrpc"), "2.0");
        assert_eq!(field(&response, "id").as_i64(), Some(self.next_id));
        response
    }

    fn read(&mut self, uri: &str) -> String {
        let response = self.request(
            "resources/read",
            &format!(r#"{{"uri":{}}}"#, json_string(uri)),
        );
        let contents = field(field(&response, "result"), "contents")
            .as_array()
            .unwrap();
        assert_eq!(contents.len(), 1, "{uri}");
        assert_eq!(string(&contents[0], "uri"), uri);
        assert_eq!(string(&contents[0], "mimeType"), MARKDOWN);
        let text = string(&contents[0], "text");
        let parts: Vec<_> = uri
            .strip_prefix("veln-doc:///package/std/snapshot/")
            .unwrap()
            .split('/')
            .collect();
        assert_eq!(parts[1], "documentation");
        assert!(text.contains(&format!("- Snapshot digest: {}\n", parts[0])));
        assert!(text.contains(&format!("- Documentation digest: {}\n", parts[2])));
        if let Some(id) = parts.get(4) {
            let kind = match parts[3] {
                "module" => "Module",
                "declaration" => "Declaration",
                other => panic!("unexpected resource kind: {other}"),
            };
            assert!(text.contains(&format!("- {kind} id: {id}\n")));
        }
        text.to_owned()
    }

    fn finish(mut self) {
        drop(self.input.take());
        let started = Instant::now();
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            assert!(started.elapsed() < TIMEOUT, "MCP should exit after EOF");
            std::thread::sleep(Duration::from_millis(10));
        };
        self.reader.take().unwrap().join().unwrap();
        assert!(
            matches!(
                self.output.try_recv(),
                Err(mpsc::TryRecvError::Disconnected)
            ),
            "unexpected extra MCP output"
        );
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        assert!(status.success(), "MCP failed: {stderr}");
        assert!(stderr.is_empty(), "unexpected MCP stderr: {stderr}");
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

fn declaration_reads() -> &'static [(&'static str, &'static str, &'static str)] {
    &[
        ("prelude", "alias Byte", "# Alias Byte"),
        ("net", "type Network", "# Type Network"),
        (
            "net",
            "function join_host_port",
            "# Function join_host_port",
        ),
        ("net", "type Address", "# Type Address"),
        ("net", "type Endpoint", "# Type Endpoint"),
        ("net", "type NetErrorKind", "# Type NetErrorKind"),
        ("net", "type NetError", "# Type NetError"),
        ("net", "type WriteAllOutcome", "# Type WriteAllOutcome"),
        (
            "net",
            "function split_host_port",
            "# Function split_host_port",
        ),
        ("net", "effect IO", "# Effect IO"),
        ("net", "handler system", "# Handler system"),
        ("net", "function write_all", "# Function write_all"),
        ("observe", "handler noop", "# Handler noop"),
        ("observe", "handler recording", "# Handler recording"),
        ("trace", "function in_span", "# Function in_span"),
        ("observe", "type Production", "# Type Production"),
        (
            "observe",
            "type ProductionHealth",
            "# Type ProductionHealth",
        ),
        (
            "observe",
            "function new_production",
            "# Function new_production",
        ),
        (
            "observe",
            "function production_health",
            "# Function production_health",
        ),
        ("observe", "handler production", "# Handler production"),
        ("prelude", "alias byte_chunk_len", "# Alias byte_chunk_len"),
        ("prelude", "alias byte_view_len", "# Alias byte_view_len"),
    ]
}
