use super::*;
use std::cell::Cell;
use std::collections::BTreeSet;
use std::rc::Rc;

use veln_language_service::{EffectiveProjectSnapshot, SourcePosition, navigate_for_rename};
use veln_source::{SourceFile, SourcePath};

use super::dependency_resources::fill_dependency_resource_capacity_completely;
use super::references_support::{
    dependency_resource_is_listed, write_workspace_with_dependency_and_sources,
};

fn rename_result(
    workspace: &TempWorkspace,
    source: &str,
    line: usize,
    column: usize,
    new_name: &str,
) -> Value {
    initialized_server(workspace).rename_tool(&json!({
        "source": source,
        "line": line,
        "column": column,
        "new_name": new_name
    }))
}

fn edits(result: &Value) -> &Vec<Value> {
    result["structuredContent"]["edits"]
        .as_array()
        .unwrap_or_else(|| panic!("rename edits missing: {result:#}"))
}

#[test]
fn rename_edit_set_matches_shared_language_service_locations() {
    let workspace = TempWorkspace::new("rename-shared-comparison");
    workspace.write("veln.toml", "");
    let main =
        "pub type Alias = Int\n\npub fn target(input: Alias) -> Alias\n  target(input)\nend\n";
    let other = "use main\n\nfn other(input: Alias) -> Alias\n  target(input)\nend\n";
    workspace.write("main.veln", main);
    workspace.write("other.veln", other);

    let snapshot = EffectiveProjectSnapshot::new(vec![
        SourceFile::new("main.veln", main),
        SourceFile::new("other.veln", other),
    ]);
    for (line, column, new_name) in [(1, 10, "Renamed"), (4, 4, "renamed")] {
        let shared = navigate_for_rename(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line,
                column,
            },
        )
        .unwrap();
        let expected = std::iter::once(&shared.definition.span)
            .chain(&shared.references)
            .map(|span| {
                (
                    crate::definition::path_to_uri(&workspace.path(span.file.as_str())),
                    span.start.line,
                    span.start.column,
                    span.end.line,
                    span.end.column,
                )
            })
            .collect::<BTreeSet<_>>();

        let actual = rename_result(&workspace, "main.veln", line, column, new_name);
        let actual = edits(&actual)
            .iter()
            .map(|edit| {
                (
                    edit["uri"].as_str().unwrap().to_owned(),
                    edit["range"]["start"]["line"].as_u64().unwrap() as usize,
                    edit["range"]["start"]["column"].as_u64().unwrap() as usize,
                    edit["range"]["end"]["line"].as_u64().unwrap() as usize,
                    edit["range"]["end"]["column"].as_u64().unwrap() as usize,
                )
            })
            .collect::<BTreeSet<_>>();

        assert_eq!(actual, expected);
    }
}

#[test]
fn rename_returns_sorted_complete_workspace_edits_for_supported_classes() {
    let workspace = TempWorkspace::new("rename-supported-classes");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "type Item\n",
            "  Value(value: Int)\n",
            "end\n\n",
            "fn convert(input: Item) -> Item\n",
            "  Value(input)\n",
            "end\n\n",
            "test converts() -> Int\n",
            "  convert(1)\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        "fn other(input: Item) -> Item\n  convert(input)\nend\n",
    );

    let cases = [
        ("type", 1, 6, "Entry", 3usize),
        ("constructor", 2, 3, "Created", 2),
        ("function", 5, 4, "adapt", 2),
        ("value binding", 6, 9, "value", 2),
        ("test declaration", 9, 6, "checks", 1),
    ];
    for (name, line, column, new_name, count) in cases {
        let result = rename_result(&workspace, "main.veln", line, column, new_name);
        assert_eq!(result["isError"], false, "{name}: {result:#}");
        assert_eq!(edits(&result).len(), count, "{name}: {result:#}");
        assert!(
            edits(&result)
                .iter()
                .all(|edit| edit["new_text"] == new_name),
            "{result:#}"
        );
        let keys = edits(&result)
            .iter()
            .map(|edit| {
                (
                    edit["uri"].as_str().unwrap(),
                    edit["range"]["start"]["line"].as_u64().unwrap(),
                    edit["range"]["start"]["column"].as_u64().unwrap(),
                    edit["range"]["end"]["line"].as_u64().unwrap(),
                    edit["range"]["end"]["column"].as_u64().unwrap(),
                )
            })
            .collect::<Vec<_>>();
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]), "{result:#}");
    }
}

#[test]
fn rename_supports_aliases_companion_private_functions_and_handler_bindings() {
    let workspace = TempWorkspace::new("rename-alias-handler-classes");
    workspace.write("veln.toml", "");
    workspace.write(
        "math.veln",
        concat!(
            "pub type Number\nend\n",
            "pub type Alias = Number\n\n",
            "fn use_alias(input: Alias) -> Alias\n  input\nend\n\n",
            "fn increment(value: Int) -> Int\n  value + 1\nend\n",
            "pub fn advance = increment\n",
            "fn use_advance() -> Int\n  advance(1)\nend\n",
        ),
    );
    workspace.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    workspace.write(
        "handler.veln",
        concat!(
            "effect Choose\n  pick(value: Bool) -> Int\nend\n\n",
            "handler choose(callback: fn(Int) -> Int) handles Choose\n",
            "  pick(value) => callback(value)\nend\n",
        ),
    );

    for (name, source, line, column, new_name) in [
        ("type alias", "math.veln", 5, 22, "RenamedAlias"),
        ("function alias", "math.veln", 14, 4, "move"),
        ("companion private", "math.test.veln", 4, 10, "step"),
        ("handler context", "handler.veln", 6, 19, "apply"),
        ("handler clause", "handler.veln", 6, 28, "input"),
    ] {
        let result = rename_result(&workspace, source, line, column, new_name);
        assert_eq!(result["isError"], false, "{name}: {result:#}");
        assert!(!edits(&result).is_empty(), "{name}: {result:#}");
    }
}

#[test]
fn rename_preserves_recovery_identity_and_same_name_edit_sets() {
    let workspace = TempWorkspace::new("rename-recovery-and-idempotence");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "type item\n  value(input: Int)\nend\n\n",
            "fn Bad() -> Int\n  Bad()\nend\n\n",
            "fn read(value: item) -> item\n  value\nend\n\n",
            "fn local(input: Int) -> Int\n  let Local = input\n  Local\nend\n",
            "\nfn clean() -> Int\n  clean()\nend\n",
        ),
    );

    for (line, column, new_name, count) in [
        (9, 19, "Entry", 3usize),
        (6, 4, "good", 2),
        (14, 8, "binding", 2),
    ] {
        let result = rename_result(&workspace, "main.veln", line, column, new_name);
        assert_eq!(result["isError"], false, "{result:#}");
        assert_eq!(edits(&result).len(), count, "{result:#}");
    }

    let same = rename_result(&workspace, "main.veln", 19, 4, "clean");
    let changed = rename_result(&workspace, "main.veln", 19, 4, "better");
    let same_ranges = edits(&same)
        .iter()
        .map(|edit| (&edit["uri"], &edit["range"]))
        .collect::<Vec<_>>();
    let changed_ranges = edits(&changed)
        .iter()
        .map(|edit| (&edit["uri"], &edit["range"]))
        .collect::<Vec<_>>();
    assert_eq!(same_ranges, changed_ranges);

    let lexical_same = rename_result(&workspace, "main.veln", 13, 10, "input");
    let lexical_changed = rename_result(&workspace, "main.veln", 13, 10, "binding");
    assert_eq!(
        edits(&lexical_same)
            .iter()
            .map(|edit| (&edit["uri"], &edit["range"]))
            .collect::<Vec<_>>(),
        edits(&lexical_changed)
            .iter()
            .map(|edit| (&edit["uri"], &edit["range"]))
            .collect::<Vec<_>>()
    );
}

#[test]
fn rename_preserves_callable_constructor_and_handler_recovery_identities() {
    let workspace = TempWorkspace::new("rename-recovery-remaining-classes");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "type item\n  value(input: Int)\nend\n\n",
            "fn read_constructor() -> item\n  value(1)\nend\n\n",
            "fn read_callback(Callback: fn() -> Int) -> Int\n  Callback\n  Callback()\nend\n\n",
            "effect Adjust\n  amount(value: Int) -> Int\nend\n\n",
            "handler adjust(Callback: fn(Int) -> Int) handles Adjust\n",
            "  amount(Value) => Callback(Value)\nend\n",
        ),
    );

    for (name, line, column, new_name, count) in [
        ("constructor", 6, 4, "Value", 2usize),
        ("callable", 10, 4, "callback", 3),
        ("handler context", 19, 20, "callback", 2),
        ("operation clause", 19, 29, "value", 2),
    ] {
        let result = rename_result(&workspace, "main.veln", line, column, new_name);
        assert_eq!(result["isError"], false, "{name}: {result:#}");
        assert_eq!(edits(&result).len(), count, "{name}: {result:#}");
    }
}

#[test]
fn rename_returns_exact_identifier_case_and_conflict_failures() {
    let workspace = TempWorkspace::new("rename-domain-failures");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "type Item\n  Value(value: Int)\nend\n\n",
            "type Status\n  Ready\nend\n\n",
            "fn convert(input: Item) -> Item\n  Value(input)\nend\n",
        ),
    );

    for bad in ["two words", "punct!", "café", "2value"] {
        let result = rename_result(&workspace, "main.veln", 1, 6, bad);
        assert_eq!(result["isError"], true, "{result:#}");
        assert_eq!(
            result["structuredContent"],
            json!({
                "code": "rename.invalid_name",
                "message": "replacement is not a valid identifier",
                "details": {"requested_name": bad}
            })
        );
    }

    for (line, column, new_name, class, required) in [
        (1, 6, "entry", "type", "ascii_uppercase"),
        (2, 3, "created", "constructor", "ascii_uppercase"),
        (9, 4, "Adapt", "function", "ascii_lowercase"),
        (10, 10, "Input", "value_binding", "ascii_lowercase"),
    ] {
        let result = rename_result(&workspace, "main.veln", line, column, new_name);
        assert_eq!(
            result["structuredContent"]["code"], "rename.invalid_case",
            "{line}:{column} {result:#}"
        );
        assert_eq!(
            result["structuredContent"]["details"],
            json!({
                "symbol_class": class,
                "requested_name": new_name,
                "required_initial": required
            })
        );
        assert!(result["structuredContent"].get("edits").is_none());
    }

    let conflict = rename_result(&workspace, "main.veln", 5, 6, "Item");
    assert_eq!(conflict["structuredContent"]["code"], "rename.conflict");
    assert_eq!(
        conflict["structuredContent"]["details"]["affected_scope"],
        json!({"kind": "module", "name": "main"})
    );
    assert!(conflict["structuredContent"].get("edits").is_none());
}

#[test]
fn rename_returns_exact_lexical_conflict_failure() {
    let workspace = TempWorkspace::new("rename-lexical-conflict");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "fn target(value: Int) -> Int\n",
            "  value\n",
            "end\n\n",
            "fn caller(value: Int) -> Int\n",
            "  let conflict = value\n",
            "  let observed = conflict\n",
            "  target(observed)\n",
            "end\n",
        ),
    );

    let lexical = rename_result(&workspace, "main.veln", 8, 4, "conflict");
    assert_eq!(lexical["structuredContent"]["code"], "rename.conflict");
    assert_eq!(
        lexical["structuredContent"]["details"]["conflicting_declaration"]["range"],
        json!({
            "start": {"line": 6, "column": 7},
            "end": {"line": 6, "column": 15}
        })
    );
    assert_eq!(
        lexical["structuredContent"]["details"]["affected_scope"]["kind"],
        "lexical"
    );
    assert_eq!(
        lexical["structuredContent"]["details"]["affected_scope"]["file"],
        "main.veln"
    );
    assert!(lexical["structuredContent"].get("edits").is_none());
}

#[test]
fn rename_returns_empty_for_unsupported_and_package_backed_selections() {
    let workspace = TempWorkspace::new("rename-unsupported-boundaries");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "schema Packet\n  value: Int\nend\n\n",
            "effect Choose\n  pick() -> Int\nend\n\n",
            "fn main() -> Int\n  dep::target()\nend\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    workspace.write("vendor/dep/dep.veln", "pub fn target() -> Int\n  1\nend\n");

    let mut server = initialized_server(&workspace);
    let before_resources = server.language_resources.list_result();
    for (line, column) in [
        (1, 5),  // module segment
        (1, 15), // package import
        (3, 8),  // schema
        (7, 8),  // effect
        (8, 3),  // effect operation
        (12, 9), // package-backed function
    ] {
        let result = server.rename_tool(&json!({
            "source":"main.veln", "line":line, "column":column, "new_name":"renamed"
        }));
        assert_eq!(result["isError"], false, "{result:#}");
        assert_eq!(edits(&result), &Vec::<Value>::new(), "{result:#}");
    }
    assert_eq!(server.language_resources.list_result(), before_resources);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}

#[test]
fn rename_is_anonymous_single_file_and_non_mutating() {
    let workspace = TempWorkspace::new("rename-anonymous-non-mutating");
    let original = "fn target(value: Int) -> Int\n  target(value - 1)\nend\n";
    workspace.write("main.veln", original);
    workspace.write("other.veln", "fn other() -> Int\n  target(1)\nend\n");
    let mut server = initialized_server(&workspace);

    let before_resources = server.language_resources.list_result();
    let result = server.rename_tool(&json!({
        "source": "main.veln", "line": 2, "column": 4, "new_name": "next"
    }));
    assert_eq!(edits(&result).len(), 2, "{result:#}");
    assert!(
        edits(&result)
            .iter()
            .all(|edit| edit["uri"].as_str().unwrap().ends_with("main.veln"))
    );
    assert_eq!(server.language_resources.list_result(), before_resources);
    assert_eq!(
        fs::read_to_string(workspace.path("main.veln")).unwrap(),
        original
    );

    let definition = server.definition_tool(&json!({
        "source": "main.veln", "line": 2, "column": 4
    }));
    assert_eq!(definition["isError"], false, "{definition:#}");
    assert_eq!(
        definition["structuredContent"]["definition"]["range"],
        json!({
            "start": {"line": 1, "column": 4},
            "end": {"line": 1, "column": 10}
        })
    );
}

#[test]
fn rename_reuses_saved_navigation_path_and_position_failures() {
    let workspace = TempWorkspace::new("rename-path-position");
    workspace.write(
        "main.veln",
        "# 😀\r\nfn target() -> Int\r\n  target()\r\nend\r\n",
    );

    let valid_non_bmp_and_crlf = rename_result(&workspace, "main.veln", 3, 4, "next");
    assert_eq!(edits(&valid_non_bmp_and_crlf).len(), 2);

    let invalid_path = rename_result(&workspace, "missing.veln", 1, 1, "next");
    assert_eq!(invalid_path["structuredContent"]["code"], "invalid_path");
    assert_eq!(invalid_path["structuredContent"]["details"], json!({}));

    let invalid_position = rename_result(&workspace, "main.veln", 1, 99, "next");
    assert_eq!(
        invalid_position["structuredContent"]["code"],
        "invalid_position"
    );
    assert_eq!(
        invalid_position["structuredContent"]["details"],
        json!({"source":"main.veln","line":1,"column":99})
    );
    assert!(invalid_position["structuredContent"].get("edits").is_none());

    let after_non_bmp = rename_result(&workspace, "main.veln", 1, 6, "next");
    assert_eq!(
        after_non_bmp["structuredContent"]["code"],
        "invalid_position"
    );

    let after_crlf = rename_result(&workspace, "main.veln", 6, 1, "next");
    assert_eq!(after_crlf["structuredContent"]["code"], "invalid_position");
}

#[test]
fn invalid_protocol_input_does_not_prevent_a_follow_up_rename() {
    let workspace = TempWorkspace::new("rename-invalid-protocol-input");
    workspace.write("main.veln", "fn target() -> Int\n  target()\nend\n");
    let mut server = initialized_server(&workspace);
    for arguments in [
        json!({"source":"main.veln","line":1,"column":4,"new_name":""}),
        json!({"source":"main.veln","line":1,"column":4}),
        json!({"source":"main.veln","line":1,"column":4,"new_name":1}),
        json!({"source":"main.veln","line":1,"column":4,"new_name":"next","extra":true}),
    ] {
        let response = server
            .handle_request(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"rename","arguments":arguments}}))
            .unwrap();
        assert_eq!(response["error"]["code"], -32602, "{response:#}");
    }
    let valid = server.rename_tool(&json!({
        "source":"main.veln","line":1,"column":4,"new_name":"next"
    }));
    assert_eq!(edits(&valid).len(), 2, "{valid:#}");
}

#[test]
fn rename_accepts_long_identifiers_before_edit_construction() {
    let workspace = TempWorkspace::new("rename-long-identifier");
    workspace.write("main.veln", "fn target() -> Int\n  target()\nend\n");
    let mut server = initialized_server(&workspace);
    let accepted_name = "a".repeat(257);
    let accepted = server
        .handle_request(json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"tools/call",
            "params":{
                "name":"rename",
                "arguments":{
                    "source":"main.veln",
                    "line":1,
                    "column":4,
                    "new_name":accepted_name
                }
            }
        }))
        .unwrap();
    assert_eq!(
        accepted["result"]["structuredContent"]["edits"]
            .as_array()
            .unwrap()
            .len(),
        2,
        "{accepted:#}"
    );
    assert!(
        accepted["result"]["structuredContent"]["edits"]
            .as_array()
            .unwrap()
            .iter()
            .all(|edit| edit["new_text"] == accepted_name),
        "{accepted:#}"
    );

    let follow_up = server.rename_tool(&json!({
        "source":"main.veln", "line":1, "column":4, "new_name":"next"
    }));
    assert_eq!(edits(&follow_up).len(), 2, "{follow_up:#}");
}

#[test]
fn rename_capture_exhaustion_preserves_state_and_allows_a_later_call() {
    let workspace = TempWorkspace::new("rename-capture-exhaustion");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", "fn target() -> Int\n  target()\nend\n");
    let mut server = initialized_server(&workspace);
    let before_resources = server.language_resources.list_result();
    let before_selection = server.selection_result();
    let cursor = server.references_tool(&json!({
        "source":"main.veln", "line":2, "column":4, "page_size":1,
        "include_declaration":true
    }))["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let attempts = Rc::new(Cell::new(0usize));
    let attempts_for_hook = Rc::clone(&attempts);
    let path = workspace.path("main.veln");
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        fs::write(
            &path,
            if attempt.is_multiple_of(2) {
                "fn target() -> Int\n  target() + 1\nend\n"
            } else {
                "fn target() -> Int\n  target()\nend\n"
            },
        )
        .unwrap();
    });

    let failed = server.rename_tool(&json!({
        "source":"main.veln", "line":2, "column":4, "new_name":"next"
    }));
    assert_eq!(failed["structuredContent"]["code"], "snapshot_changed");
    assert!(failed["structuredContent"].get("edits").is_none());
    assert_eq!(attempts.get(), 3);
    assert_eq!(server.language_resources.list_result(), before_resources);
    assert_eq!(server.selection_result(), before_selection);

    let continuation = server.references_tool(&json!({"cursor": cursor}));
    assert_eq!(continuation["isError"], false, "{continuation:#}");
    assert_eq!(
        continuation["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    drop(_hook);
    workspace.write("main.veln", "fn target() -> Int\n  target()\nend\n");
    let valid = server.rename_tool(&json!({
        "source":"main.veln", "line":2, "column":4, "new_name":"next"
    }));
    assert_eq!(edits(&valid).len(), 2, "{valid:#}");
}

#[test]
fn rename_result_is_independent_of_full_retained_resource_capacity() {
    let workspace = TempWorkspace::new("rename-full-resource-capacity");
    write_workspace_with_dependency_and_sources(
        &workspace,
        "fn target() -> Int\n  target()\nend\n\nfn occupied() -> Int\n  1\nend\n",
        None,
    );
    let mut server = initialized_server_with_embedded_resources(&workspace);
    let arguments = [
        json!({"source":"main.veln", "line":2, "column":4, "new_name":"next"}),
        json!({"source":"main.veln", "line":2, "column":2, "new_name":"next"}),
        json!({"source":"main.veln", "line":2, "column":4, "new_name":"two words"}),
        json!({"source":"main.veln", "line":2, "column":4, "new_name":"Next"}),
        json!({"source":"main.veln", "line":2, "column":4, "new_name":"occupied"}),
    ];
    let before = arguments
        .iter()
        .map(|arguments| server.rename_tool(arguments))
        .collect::<Vec<_>>();
    fill_dependency_resource_capacity_completely(&mut server);
    let full_resources = server.language_resources.list_result();

    let after = arguments
        .iter()
        .map(|arguments| server.rename_tool(arguments))
        .collect::<Vec<_>>();

    assert_eq!(after, before);
    assert_eq!(server.language_resources.list_result(), full_resources);
}
