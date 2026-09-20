use super::*;
use std::cell::Cell;
use std::rc::Rc;

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
    for (line, column) in [(3, 8), (7, 8), (12, 9)] {
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
    workspace.write("main.veln", "fn 😀target() -> Int\r\n  1\r\nend\r\n");

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
fn rename_capture_exhaustion_preserves_state_and_allows_a_later_call() {
    let workspace = TempWorkspace::new("rename-capture-exhaustion");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", "fn target() -> Int\n  target()\nend\n");
    let mut server = initialized_server(&workspace);
    let before_resources = server.language_resources.list_result();
    let before_selection = server.selection_result();
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
        "fn target() -> Int\n  target()\nend\n",
        None,
    );
    let mut server = initialized_server_with_embedded_resources(&workspace);
    let arguments = json!({
        "source":"main.veln", "line":2, "column":4, "new_name":"next"
    });
    let before = server.rename_tool(&arguments);
    fill_dependency_resource_capacity_completely(&mut server);
    let full_resources = server.language_resources.list_result();

    let after = server.rename_tool(&arguments);

    assert_eq!(after, before);
    assert_eq!(server.language_resources.list_result(), full_resources);
}
