use super::*;
use std::env;
use veln_diagnostics::Severity;
use veln_diagnostics::{DiagnosticKind, JsonValue};
use veln_project::materialized_git_repository_root;
use veln_source::SourceSpan;

fn standard_library_virtual_document_project() -> TempProject {
    let project = TempProject::new("standard-library-virtual-document");
    project.write(
        "main.veln",
        concat!(
            "use http2::diagnostic from \"std\"\n\n",
            "pub fn implicit() -> Result<Byte, String>\n",
            "  byte(1)\n",
            "end\n\n",
            "pub fn qualified() -> Result<Byte, String>\n",
            "  prelude::byte(1)\n",
            "end\n\n",
            "pub fn parameter_shadow(byte: fn(Int) -> Result<Byte, String>) -> Result<Byte, String>\n",
            "  byte(1)\n",
            "end\n\n",
            "pub fn local_shadow() -> Result<Byte, String>\n",
            "  let byte: fn(Int) -> Result<Byte, String> = prelude::byte\n",
            "  byte(1)\n",
            "end\n\n",
            "pub fn imported() -> Result<(), RuntimeDiagnostic>\n",
            "  http2::diagnostic::protocol_invalid_frame_kind(0, 0, 0, 0, \"open\", \"rule\", byte_view(byte_chunk([]), ByteOffset(0), ByteCount(0)))\n",
            "end\n\n",
            "pub fn private_helper() -> Vec<Int>\n",
            "  prelude::vec_append([], 1)\n",
            "end\n",
        ),
    );
    project
}

fn assert_standard_prelude_navigation(server: &mut Server, main_uri: &str) -> String {
    let implicit = server.handle_message(&definition_request(main_uri, 3, 4));
    let qualified = server.handle_message(&definition_request(main_uri, 7, 12));
    let shadowed_parameter = server.handle_message(&definition_request(main_uri, 11, 2));
    let shadowed_local = server.handle_message(&definition_request(main_uri, 16, 2));
    let prelude_uri = package_virtual_definition_uri(&implicit[0], "std", "prelude.veln");

    assert_eq!(
        extract_string_field(&qualified[0], "uri"),
        Some(prelude_uri.clone())
    );
    assert!(
        implicit[0].contains(
            r#""range":{"start":{"line":97,"character":7},"end":{"line":97,"character":11}}"#
        ),
        "{}",
        implicit[0]
    );
    assert_null_result(&shadowed_parameter[0]);
    assert_null_result(&shadowed_local[0]);
    prelude_uri
}

fn assert_standard_import_navigation(server: &mut Server, main_uri: &str) {
    let imported = server.handle_message(&definition_request(main_uri, 20, 31));
    let diagnostic_uri =
        package_virtual_definition_uri(&imported[0], "std", "http2/diagnostic.veln");
    assert!(diagnostic_uri.contains("/http2/diagnostic.veln"));
}

fn package_virtual_definition_uri(response: &str, package: &str, source_path: &str) -> String {
    let uri = extract_string_field(response, "uri").unwrap();
    assert!(
        uri.starts_with(&format!("veln-pkg:///{package}/snapshot/")),
        "{response}"
    );
    assert!(uri.ends_with(&format!("/{source_path}")), "{response}");
    uri
}

fn assert_virtual_document_text(server: &mut Server, id: &str, uri: &str, expected: &str) {
    let read = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"veln/virtualDocument","params":{{"uri":"{uri}"}}}}"#
    ));
    assert_eq!(
        read,
        [response(id, &format!(r#""{}""#, escape_json(expected)))]
    );
}

fn standard_source_text(path: &str) -> &'static str {
    veln_stdlib::package_bundle()
        .files
        .iter()
        .find(|file| file.path == path)
        .unwrap()
        .text
}

fn assert_null_result(response: &str) {
    assert!(response.contains(r#""result":null"#), "{response}");
}

fn assert_empty_result_array(response: &str) {
    assert!(response.contains(r#""result":[]"#), "{response}");
}

fn assert_invalid_params(response: &str) {
    assert!(response.contains(r#""code":-32602"#), "{response}");
}

fn assert_contains_json(response: &str, expected: &str) {
    assert!(response.contains(expected), "{response}");
}

fn assert_not_contains_json(response: &str, rejected: &str) {
    assert!(!response.contains(rejected), "{response}");
}

fn assert_single_response<'a>(responses: &'a [String], expected: &str) -> &'a str {
    assert_eq!(responses.len(), 1);
    assert_contains_json(&responses[0], expected);
    &responses[0]
}

fn publish_for_uri<'a>(responses: &'a [String], uri: &str) -> &'a str {
    responses
        .iter()
        .find(|response| {
            response.contains(r#""method":"textDocument/publishDiagnostics""#)
                && response.contains(&format!(r#""uri":"{}""#, escape_json(uri)))
        })
        .map(String::as_str)
        .unwrap_or_else(|| panic!("expected publish diagnostics for {uri}: {responses:#?}"))
}

fn companion_private_function_project(name: &str) -> TempProject {
    let project = TempProject::new(name);
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  increment(value - 1)\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        concat!(
            "use math\n",
            "\n",
            "fn increment(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "\n",
            "test increment_test() -> Int\n",
            "  math::increment(1)\n",
            "end\n",
            "\n",
            "test local_increment_test() -> Int\n",
            "  increment(1)\n",
            "end\n",
        ),
    );
    project
}

fn initialize_request(root_uri: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
    )
}

fn definition_request(uri: &str, line: usize, character: usize) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/definition","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}}}}}}"#
    )
}

fn references_request(uri: &str, line: usize, character: usize) -> String {
    references_request_with_declaration(uri, line, character, true)
}

fn references_request_with_declaration(
    uri: &str,
    line: usize,
    character: usize,
    include_declaration: bool,
) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/references","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}},"context":{{"includeDeclaration":{include_declaration}}}}}}}"#
    )
}

fn prepare_rename_request(uri: &str, line: usize, character: usize) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/prepareRename","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}}}}}}"#
    )
}

fn rename_request(uri: &str, line: usize, character: usize, new_name: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/rename","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}},"newName":"{new_name}"}}}}"#
    )
}

fn semantic_tokens_request(uri: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/semanticTokens/full","params":{{"textDocument":{{"uri":"{uri}"}}}}}}"#
    )
}

struct TempProject {
    root: PathBuf,
}

impl TempProject {
    fn new(name: &str) -> Self {
        let root = env::temp_dir().join(format!(
            "veln-lsp-{name}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&root).expect("temp project should be created");
        Self { root }
    }

    fn write(&self, path: &str, contents: &str) {
        let path = self.root.join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("fixture parent should be created");
        }
        fs::write(path, contents).expect("fixture source should be written");
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should produce a temp suffix")
        .as_nanos()
}
