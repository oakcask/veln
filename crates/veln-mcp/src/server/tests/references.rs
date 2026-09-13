use super::*;
use std::cell::Cell;
use std::rc::Rc;
use veln_project::PackageSnapshotSource;

struct WorkspaceSymbolCase {
    name: &'static str,
    files: Vec<(&'static str, &'static str)>,
    source: &'static str,
    line: usize,
    column: usize,
    ranges: Vec<(&'static str, usize, usize, usize, usize)>,
}

fn assert_workspace_symbol_cases(cases: impl IntoIterator<Item = WorkspaceSymbolCase>) {
    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        for (path, text) in case.files {
            workspace.write(path, text);
        }
        let result = references_result(&workspace, case.source, case.line, case.column);
        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_eq!(
            result["structuredContent"]["scope"]["project_wide"], true,
            "{}: {result:#}",
            case.name
        );
        assert_reference_ranges(&result, &case.ranges, case.name);
    }
}

#[test]
fn references_return_sorted_project_function_locations_and_scope() {
    let workspace = TempWorkspace::new("references-project");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "type Token\n",
            "  Helper\n",
            "end\n\n",
            "fn helper(value: Int) -> Int\n",
            "  helper(value - 1)\n",
            "end\n\n",
            "fn main() -> Int\n",
            "  helper(1)\n",
            "end\n\n",
            "fn unrelated() -> Token\n",
            "  Helper()\n",
            "end\n",
        ),
    );

    let result = references_result(&workspace, "main.veln", 10, 4);
    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "project",
            "generation": 0,
            "project": ".",
            "project_wide": true
        })
    );
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert_eq!(references.len(), 2, "{result:#}");
    assert!(
        references[0]["uri"]
            .as_str()
            .unwrap()
            .ends_with("main.veln"),
        "{result:#}"
    );
    assert_eq!(
        references[0]["range"],
        json!({"start": {"line": 6, "column": 3}, "end": {"line": 6, "column": 9}})
    );
    assert_eq!(
        references[1]["range"],
        json!({"start": {"line": 10, "column": 3}, "end": {"line": 10, "column": 9}})
    );

    let constructor = references_result(&workspace, "main.veln", 14, 4);
    assert_eq!(constructor["isError"], false, "{constructor:#}");
    assert_reference_ranges(
        &constructor,
        &[("main.veln", 14, 3, 14, 9)],
        "constructor support",
    );
}

#[test]
fn references_resolve_types_and_constructors() {
    let cases = [
        WorkspaceSymbolCase {
            name: "type selected at declaration",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "type Item\n",
                        "  Some(Int)\n",
                        "  None\n",
                        "end\n\n",
                        "fn make() -> Item\n",
                        "  Item::Some(1)\n",
                        "end\n\n",
                        "fn observe(input: Item) -> Int\n",
                        "  match input\n",
                        "    Item::None => 0\n",
                        "    Item::Some(value) => value\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 1,
            column: 6,
            ranges: vec![
                ("main.veln", 6, 14, 6, 18),
                ("main.veln", 7, 3, 7, 7),
                ("main.veln", 10, 19, 10, 23),
                ("main.veln", 12, 5, 12, 9),
                ("main.veln", 13, 5, 13, 9),
            ],
        },
        WorkspaceSymbolCase {
            name: "type selected at bare use",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "type Item\n",
                        "  Some(Int)\n",
                        "end\n\n",
                        "type Other\n",
                        "end\n\n",
                        "fn read(input: Item) -> Item\n",
                        "  input\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 8,
            column: 19,
            ranges: vec![("main.veln", 8, 16, 8, 20), ("main.veln", 8, 25, 8, 29)],
        },
        WorkspaceSymbolCase {
            name: "type selected at qualified use with collisions",
            files: vec![
                ("veln.toml", ""),
                (
                    "helper.veln",
                    "pub type Item\n  pub Ready(Int)\nend\n\npub type Other\nend\n",
                ),
                (
                    "main.veln",
                    concat!(
                        "use helper\n\n",
                        "type Item\n",
                        "end\n\n",
                        "fn make() -> helper::Item\n",
                        "  helper::Item::Ready(1)\n",
                        "end\n\n",
                        "fn read(input: helper::Item) -> helper::Item\n",
                        "  input\n",
                        "end\n",
                    ),
                ),
                (
                    "other.veln",
                    "pub type Item\nend\n\nfn read(input: Item) -> Item\n  input\nend\n",
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 22,
            ranges: vec![
                ("main.veln", 6, 22, 6, 26),
                ("main.veln", 7, 11, 7, 15),
                ("main.veln", 10, 24, 10, 28),
                ("main.veln", 10, 41, 10, 45),
            ],
        },
        WorkspaceSymbolCase {
            name: "constructor selected at declaration",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "type Status\n",
                        "  Ready\n",
                        "  Waiting\n",
                        "end\n\n",
                        "fn ready() -> Status\n",
                        "  Ready\n",
                        "end\n\n",
                        "fn observe(status: Status) -> Bool\n",
                        "  match status\n",
                        "    Ready => true\n",
                        "    Waiting => false\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 2,
            column: 4,
            ranges: vec![("main.veln", 7, 3, 7, 8), ("main.veln", 12, 5, 12, 10)],
        },
        WorkspaceSymbolCase {
            name: "constructor selected at qualified call with collisions",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "type Left\n",
                        "  Thing(Int)\n",
                        "end\n\n",
                        "type Right\n",
                        "  Thing(Int)\n",
                        "end\n\n",
                        "effect Task\n",
                        "  Thing() -> Int\n",
                        "end\n\n",
                        "fn make() -> Left\n",
                        "  Left::Thing(1)\n",
                        "end\n\n",
                        "fn read(input: Left) -> Int\n",
                        "  match input\n",
                        "    Left::Thing(value) => value\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 14,
            column: 9,
            ranges: vec![("main.veln", 14, 9, 14, 14), ("main.veln", 19, 11, 19, 16)],
        },
        WorkspaceSymbolCase {
            name: "constructor selected at nullary expression",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "type Status\n",
                        "  Ready\n",
                        "  Waiting\n",
                        "end\n\n",
                        "fn ready() -> Status\n",
                        "  Ready\n",
                        "end\n\n",
                        "fn observe(status: Status) -> Bool\n",
                        "  match status\n",
                        "    Ready => true\n",
                        "    Waiting => false\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 7,
            column: 4,
            ranges: vec![("main.veln", 7, 3, 7, 8), ("main.veln", 12, 5, 12, 10)],
        },
        WorkspaceSymbolCase {
            name: "constructor selected at bare pattern",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "type Status\n",
                        "  Ready\n",
                        "  Waiting\n",
                        "end\n\n",
                        "fn ready() -> Status\n",
                        "  Ready\n",
                        "end\n\n",
                        "fn observe(status: Status) -> Bool\n",
                        "  match status\n",
                        "    Ready => true\n",
                        "    Waiting => false\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 12,
            column: 7,
            ranges: vec![("main.veln", 7, 3, 7, 8), ("main.veln", 12, 5, 12, 10)],
        },
        WorkspaceSymbolCase {
            name: "constructor selected through imported alias",
            files: vec![
                ("veln.toml", ""),
                ("app/math.veln", "pub type Result\n  pub Done\nend\n"),
                (
                    "main.veln",
                    concat!(
                        "use app::math\n\n",
                        "fn make() -> math::Result\n",
                        "  math::Result::Done\n",
                        "end\n\n",
                        "fn read(input: math::Result) -> Bool\n",
                        "  match input\n",
                        "    math::Result::Done => true\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 4,
            column: 17,
            ranges: vec![("main.veln", 4, 17, 4, 21), ("main.veln", 9, 19, 9, 23)],
        },
    ];

    assert_workspace_symbol_cases(cases);
}

#[test]
fn references_return_workspace_schema_operation_locations_and_scope() {
    let workspace = TempWorkspace::new("references-workspace-schema");
    workspace.write("veln.toml", "");
    workspace.write(
        "app/wire.veln",
        concat!(
            "pub schema Packet\n",
            "  format binary\n",
            "  value: UInt8\n",
            "end\n\n",
            "fn local(view: ByteView, packet: {value: Int}) -> ()\n",
            "  let decoded = decode Packet from view at byte_offset(0)?\n",
            "  let encoded = encode Packet from packet\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        concat!(
            "use app::wire\n\n",
            "fn imported(view: ByteView, packet: {value: Int}) -> ()\n",
            "  let qualified = decode app::wire::Packet from view at byte_offset(0)?\n",
            "  let alias_qualified = encode wire::Packet from packet\n",
            "  let bare_decode = decode Packet from view at byte_offset(0)?\n",
            "  let bare_encode = encode Packet from packet\n",
            "end\n",
        ),
    );

    let result = references_result(&workspace, "app/wire.veln", 1, 12);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "project",
            "generation": 0,
            "project": ".",
            "project_wide": true
        })
    );
    assert_reference_ranges(
        &result,
        &[
            ("app/wire.veln", 7, 24, 7, 30),
            ("app/wire.veln", 8, 24, 8, 30),
            ("other.veln", 4, 37, 4, 43),
            ("other.veln", 5, 38, 5, 44),
        ],
        "workspace schema operation references",
    );
}

#[test]
fn references_keep_workspace_schema_identity_visibility_and_companion_boundaries() {
    let cases = [
        WorkspaceSymbolCase {
            name: "schema import visibility and shadowing",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "pub schema Packet\n  value: Int\nend\n\nschema Hidden\n  value: Int\nend\n",
                ),
                (
                    "other.veln",
                    concat!(
                        "use main\n\n",
                        "schema Packet\n",
                        "  local: Int\n",
                        "end\n\n",
                        "fn read(view: ByteView, packet: {value: Int}) -> ()\n",
                        "  let local = decode Packet from view at byte_offset(0)?\n",
                        "  let selected = encode main::Packet from packet\n",
                        "  let hidden = decode main::Hidden from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 1,
            column: 12,
            ranges: vec![("other.veln", 9, 31, 9, 37)],
        },
        WorkspaceSymbolCase {
            name: "schema exact companion private access",
            files: vec![
                ("veln.toml", ""),
                ("main.veln", "schema PrivatePacket\n  value: Int\nend\n"),
                (
                    "main.test.veln",
                    concat!(
                        "use main\n\n",
                        "test companion(view: ByteView, packet: {value: Int}) -> ()\n",
                        "  let decoded = decode main::PrivatePacket from view at byte_offset(0)?\n",
                        "  let encoded = encode main::PrivatePacket from packet\n",
                        "end\n",
                    ),
                ),
                (
                    "other.test.veln",
                    concat!(
                        "use main\n\n",
                        "test unrelated(view: ByteView) -> ()\n",
                        "  decode main::PrivatePacket from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 1,
            column: 8,
            ranges: vec![
                ("main.test.veln", 4, 30, 4, 43),
                ("main.test.veln", 5, 30, 5, 43),
            ],
        },
        WorkspaceSymbolCase {
            name: "schema collisions",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "schema packet\n",
                        "  value: Int\n",
                        "end\n\n",
                        "fn packet() -> Int\n",
                        "  1\n",
                        "end\n\n",
                        "type Holder\n",
                        "  packet\n",
                        "end\n\n",
                        "effect packet\n",
                        "  packet() -> Int\n",
                        "end\n\n",
                        "fn read(packet: {value: Int}) -> () effects [packet]\n",
                        "  let packet = packet()\n",
                        "  let encoded = encode packet from packet\n",
                        "  let field = packet.value\n",
                        "  # packet in a comment\n",
                        "  \"packet\"\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 1,
            column: 8,
            ranges: vec![("main.veln", 19, 24, 19, 30)],
        },
    ];

    assert_workspace_symbol_cases(cases);
}

#[test]
fn references_resolve_callable_and_local_bindings() {
    let cases = [
        WorkspaceSymbolCase {
            name: "callable value binding",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "type Item\n",
                        "  byte(value: Int)\n",
                        "end\n\n",
                        "fn caller(byte: fn() -> Int) -> Int\n",
                        "  byte()\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 4,
            ranges: vec![("main.veln", 6, 3, 6, 7)],
        },
        WorkspaceSymbolCase {
            name: "function parameter with shadowing and field collision",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "fn read(value: Int, record: {value: Int}) -> Int\n",
                        "  let value = value\n",
                        "  record.value + value\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 2,
            column: 15,
            ranges: vec![("main.veln", 2, 15, 2, 20)],
        },
        WorkspaceSymbolCase {
            name: "result binding",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "fn read(value: Int) -> output: Int\n",
                        "  ensure output >= value\n",
                        "  value\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 2,
            column: 11,
            ranges: vec![("main.veln", 2, 10, 2, 16)],
        },
        WorkspaceSymbolCase {
            name: "local let binding starts after initializer",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "fn read() -> Int\n",
                        "  let worker = 1\n",
                        "  worker\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 3,
            column: 4,
            ranges: vec![("main.veln", 3, 3, 3, 9)],
        },
        WorkspaceSymbolCase {
            name: "local pattern binding",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "type Item\n",
                        "  Some(Int)\n",
                        "end\n\n",
                        "fn read(item: Item) -> Int\n",
                        "  let Some(value) = item\n",
                        "  value\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 7,
            column: 4,
            ranges: vec![("main.veln", 7, 3, 7, 8)],
        },
        WorkspaceSymbolCase {
            name: "match arm pattern binding",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "type Item\n",
                        "  Some(Int)\n",
                        "end\n\n",
                        "fn read(item: Item) -> Int\n",
                        "  match item\n",
                        "    Some(value) => value\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 7,
            column: 20,
            ranges: vec![("main.veln", 7, 20, 7, 25)],
        },
        WorkspaceSymbolCase {
            name: "satisfy candidate binding",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "fn read(limit: Int) -> Int\n",
                        "  _value satisfy candidate => candidate <= limit\n",
                        "  limit\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 2,
            column: 32,
            ranges: vec![("main.veln", 2, 31, 2, 40)],
        },
    ];

    assert_workspace_symbol_cases(cases);
}

#[test]
fn references_resolve_handler_bindings() {
    let cases = [
        WorkspaceSymbolCase {
            name: "handler context parameter",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "effect Adjust\n",
                        "  amount(value: Int) -> Int\n",
                        "  echo(value: Int) -> Int\n",
                        "end\n\n",
                        "handler adjust(callback: fn(Int) -> Int) handles Adjust\n",
                        "  amount(value) => callback(value)\n",
                        "  echo(value) => callback(value)\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 7,
            column: 22,
            ranges: vec![("main.veln", 7, 20, 7, 28), ("main.veln", 8, 18, 8, 26)],
        },
        WorkspaceSymbolCase {
            name: "handler context callable parameter with inner shadowing",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "fn callback(value: Int) -> Int\n",
                        "  value\n",
                        "end\n\n",
                        "effect Adjust\n",
                        "  amount(value: Int) -> Int\n",
                        "  reset(value: Int) -> Int\n",
                        "end\n\n",
                        "handler adjust(callback: fn(Int) -> Int) handles Adjust\n",
                        "  amount(value) => callback(value) + callback(1)\n",
                        "  reset(callback) => callback\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 11,
            column: 22,
            ranges: vec![("main.veln", 11, 20, 11, 28), ("main.veln", 11, 38, 11, 46)],
        },
        WorkspaceSymbolCase {
            name: "handler operation clause parameter",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "effect Choose\n",
                        "  pick(value: Bool) -> Int\n",
                        "end\n\n",
                        "handler choose() handles Choose\n",
                        "  pick(value) => match value\n",
                        "    true => value\n",
                        "    value => value\n",
                        "    false => record.value\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 7,
            column: 16,
            ranges: vec![("main.veln", 6, 24, 6, 29), ("main.veln", 7, 13, 7, 18)],
        },
        WorkspaceSymbolCase {
            name: "handler operation callable clause parameter",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "effect Run\n",
                        "  call(action: fn() -> Int) -> Int\n",
                        "end\n\n",
                        "handler run() handles Run\n",
                        "  call(action) => action()\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 20,
            ranges: vec![("main.veln", 6, 19, 6, 25)],
        },
        WorkspaceSymbolCase {
            name: "handler operation clause parameter with inner shadowing",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "effect Choose\n",
                        "  pick(value: Bool) -> Int\n",
                        "end\n\n",
                        "handler choose() handles Choose\n",
                        "  pick(value) => match value\n",
                        "    true => value\n",
                        "    value => value\n",
                        "    false => value\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 9,
            column: 16,
            ranges: vec![
                ("main.veln", 6, 24, 6, 29),
                ("main.veln", 7, 13, 7, 18),
                ("main.veln", 9, 14, 9, 19),
            ],
        },
    ];

    assert_workspace_symbol_cases(cases);
}

#[test]
fn references_return_direct_dependency_function_locations_from_saved_project() {
    let workspace = TempWorkspace::new("references-dependency-function");
    write_dependency_reference_workspace(&workspace, "path", "vendor/dep", None);

    let result = references_result(&workspace, "main.veln", 4, 10);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "project",
            "generation": 0,
            "project": ".",
            "project_wide": true
        })
    );
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 4, 9, 4, 17),
            ("main.veln", 8, 40, 8, 48),
            ("main.veln", 9, 18, 9, 26),
        ],
        "dependency function references",
    );
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert!(references.iter().all(|reference| {
        reference["uri"].as_str().unwrap().starts_with("file://")
            && !reference["uri"].as_str().unwrap().contains("veln-pkg:")
    }));
}

#[test]
fn references_accept_direct_dependency_function_source_forms() {
    struct Case {
        name: &'static str,
        field: &'static str,
        source: &'static str,
        selector: Option<&'static str>,
    }

    let cases = [
        Case {
            name: "path",
            field: "path",
            source: "vendor/path-dep",
            selector: None,
        },
        Case {
            name: "vendor",
            field: "vendor",
            source: "vendor/vendor-dep",
            selector: None,
        },
        Case {
            name: "mirror",
            field: "mirror",
            source: "vendor/mirror-dep",
            selector: None,
        },
        Case {
            name: "local git",
            field: "git",
            source: "vendor/git-dep",
            selector: Some("rev = \"abc123\""),
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        write_dependency_reference_workspace(&workspace, case.field, case.source, case.selector);

        let result = references_result(&workspace, "main.veln", 4, 10);

        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_reference_ranges(
            &result,
            &[
                ("main.veln", 4, 9, 4, 17),
                ("main.veln", 8, 40, 8, 48),
                ("main.veln", 9, 18, 9, 26),
            ],
            case.name,
        );
    }
}

#[test]
fn references_keep_direct_dependency_function_identity_boundaries() {
    let workspace = TempWorkspace::new("references-dependency-function-boundaries");
    workspace.write(
        "veln.toml",
        concat!(
            "[dependencies.\"example/dep\"]\n",
            "path = \"vendor/dep\"\n",
            "\n",
            "[dependencies.\"other/dep\"]\n",
            "path = \"vendor/other\"\n",
        ),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use lib::math from \"example/dep\"\n",
            "use other_math from \"other/dep\"\n\n",
            "pub fn read(record: {field: Int}, value: Int) -> Int\n",
            "  let local = value\n",
            "  math::target(value)\n",
            "  other_math::target(value)\n",
            "  record.field + local\n",
            "end\n",
        ),
    );
    workspace.write(
        "other/main.veln",
        concat!(
            "use lib::math from \"example/dep\"\n\n",
            "pub fn read(value: Int) -> Int\n",
            "  math::target(value)\n",
            "end\n",
        ),
    );
    workspace.write("other/veln.toml", "");
    write_dependency_package(&workspace, "vendor/dep", "example/dep", "lib/math.veln");
    write_dependency_package(&workspace, "vendor/other", "other/dep", "other_math.veln");

    let result = references_result(&workspace, "main.veln", 6, 10);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_reference_ranges(
        &result,
        &[("main.veln", 6, 9, 6, 15)],
        "dependency function identity",
    );
}

#[test]
fn references_return_direct_dependency_type_locations_from_saved_project() {
    let workspace = TempWorkspace::new("references-dependency-type");
    workspace.write(
        "veln.toml",
        concat!(
            "[dependencies.\"example/dep\"]\n",
            "path = \"vendor/dep\"\n",
            "\n",
            "[dependencies.\"other/dep\"]\n",
            "path = \"vendor/other\"\n",
        ),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use model from \"example/dep\"\n",
            "use other_model from \"other/dep\"\n\n",
            "type Item\n",
            "end\n\n",
            "type Box\n",
            "  Wrap(model::Item)\n",
            "end\n\n",
            "pub type Alias = model::Item\n\n",
            "fn make(input: model::Item) -> model::Item\n",
            "  model::Item::Ready(1)\n",
            "end\n\n",
            "fn other(input: other_model::Item) -> Item\n",
            "  \"Item\"\n",
            "end\n\n",
            "# Item in a comment is not a type reference.\n",
            "fn wrapped(items: Vec<model::Item>) -> Vec<model::Item>\n",
            "  items\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        concat!(
            "use model from \"example/dep\"\n\n",
            "fn second(input: model::Item) -> model::Item\n",
            "  input\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"model.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/model.veln",
        "pub type Item\n  pub Ready(Int)\nend\n\nfn package_body(input: Item) -> Item\n  input\nend\n",
    );
    workspace.write(
        "vendor/other/veln.toml",
        "[package]\nname = \"other/dep\"\n\n[lib]\nexports = [\"model.veln\"]\n",
    );
    workspace.write("vendor/other/model.veln", "pub type Item\nend\n");

    let result = references_result(&workspace, "main.veln", 13, 23);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "project",
            "generation": 0,
            "project": ".",
            "project_wide": true
        })
    );
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 8, 15, 8, 19),
            ("main.veln", 11, 25, 11, 29),
            ("main.veln", 13, 23, 13, 27),
            ("main.veln", 13, 39, 13, 43),
            ("main.veln", 14, 10, 14, 14),
            ("main.veln", 22, 30, 22, 34),
            ("main.veln", 22, 51, 22, 55),
            ("other.veln", 3, 25, 3, 29),
            ("other.veln", 3, 41, 3, 45),
        ],
        "dependency type references",
    );
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert!(references.iter().all(|reference| {
        reference["uri"].as_str().unwrap().starts_with("file://")
            && !reference["uri"].as_str().unwrap().contains("veln-pkg:")
            && !reference["uri"].as_str().unwrap().contains("vendor/dep")
    }));
}

#[test]
fn references_return_direct_dependency_type_alias_locations_from_saved_project() {
    let workspace = TempWorkspace::new("references-dependency-type-alias");
    workspace.write(
        "veln.toml",
        concat!(
            "[dependencies.\"example/dep\"]\n",
            "path = \"vendor/dep\"\n",
            "\n",
            "[dependencies.\"other/dep\"]\n",
            "path = \"vendor/other\"\n",
        ),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use lib::model from \"example/dep\"\n",
            "use other_model from \"other/dep\"\n\n",
            "type Box\n",
            "  Wrap(model::Alias)\n",
            "end\n\n",
            "pub type LocalAlias = model::Alias\n\n",
            "fn make(input: model::Alias, boxed: Vec<lib::model::Alias>) -> model::Alias\n",
            "  model::Alias::Ready(1)\n",
            "end\n\n",
            "fn other(input: other_model::Alias, target: model::Item) -> lib::model::Item\n",
            "  \"Alias\"\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        concat!(
            "use lib::model from \"example/dep\"\n\n",
            "fn second(input: model::Alias) -> lib::model::Alias\n",
            "  input\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"lib/model.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/lib/model.veln",
        "pub type Item\n  pub Ready(Int)\nend\n\npub type Alias = Item\n",
    );
    workspace.write(
        "vendor/other/veln.toml",
        "[package]\nname = \"other/dep\"\n\n[lib]\nexports = [\"other_model.veln\"]\n",
    );
    workspace.write(
        "vendor/other/other_model.veln",
        "pub type Item\nend\n\npub type Alias = Item\n",
    );

    let result = references_result(&workspace, "main.veln", 10, 24);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "project",
            "generation": 0,
            "project": ".",
            "project_wide": true
        })
    );
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 5, 15, 5, 20),
            ("main.veln", 8, 30, 8, 35),
            ("main.veln", 10, 23, 10, 28),
            ("main.veln", 10, 53, 10, 58),
            ("main.veln", 10, 71, 10, 76),
            ("main.veln", 11, 10, 11, 15),
            ("other.veln", 3, 25, 3, 30),
            ("other.veln", 3, 47, 3, 52),
        ],
        "dependency type alias references",
    );
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert!(references.iter().all(|reference| {
        reference["uri"].as_str().unwrap().starts_with("file://")
            && !reference["uri"].as_str().unwrap().contains("veln-pkg:")
            && !reference["uri"].as_str().unwrap().contains("vendor/dep")
    }));

    let target = references_result(&workspace, "main.veln", 14, 54);
    assert_eq!(target["isError"], false, "{target:#}");
    assert_reference_ranges(
        &target,
        &[("main.veln", 14, 52, 14, 56), ("main.veln", 14, 73, 14, 77)],
        "dependency type target references",
    );
}

#[test]
fn references_return_direct_dependency_constructor_locations_from_saved_project() {
    let workspace = TempWorkspace::new("references-dependency-constructor");
    workspace.write(
        "veln.toml",
        concat!(
            "[dependencies.\"example/dep\"]\n",
            "path = \"vendor/dep\"\n",
            "\n",
            "[dependencies.\"other/dep\"]\n",
            "path = \"vendor/other\"\n",
        ),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use model from \"example/dep\"\n",
            "use other_model from \"other/dep\"\n\n",
            "type Local\n",
            "  Ready(Int)\n",
            "end\n\n",
            "fn make(input: Int) -> model::Item\n",
            "  model::Item::Ready(input)\n",
            "end\n\n",
            "fn alias_route(input: Int) -> model::Alias\n",
            "  model::Alias::Ready(input)\n",
            "end\n\n",
            "fn collisions(record: {Ready: Int}, Ready: Int) -> Int\n",
            "  other_model::Ready(1)\n",
            "  Local::Ready(2)\n",
            "  record.Ready + Ready\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        concat!(
            "use model from \"example/dep\"\n\n",
            "fn other(input: model::Item) -> Int\n",
            "  match input\n",
            "    Ready(value) => value\n",
            "  end\n",
            "end\n\n",
            "fn qualified() -> model::Item\n",
            "  model::Ready(1)\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"model.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/model.veln",
        "pub type Item\n  pub Ready(Int)\nend\n\npub type Alias = Item\n\nfn package_body() -> Item\n  Ready(1)\nend\n",
    );
    workspace.write(
        "vendor/other/veln.toml",
        "[package]\nname = \"other/dep\"\n\n[lib]\nexports = [\"model.veln\"]\n",
    );
    workspace.write(
        "vendor/other/model.veln",
        "pub type Item\n  pub Ready(Int)\nend\n",
    );

    let result = references_result(&workspace, "main.veln", 9, 17);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "project",
            "generation": 0,
            "project": ".",
            "project_wide": true
        })
    );
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 9, 16, 9, 21),
            ("other.veln", 5, 5, 5, 10),
            ("other.veln", 10, 10, 10, 15),
        ],
        "dependency constructor references",
    );
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert!(references.iter().all(|reference| {
        reference["uri"].as_str().unwrap().starts_with("file://")
            && !reference["uri"].as_str().unwrap().contains("veln-pkg:")
            && !reference["uri"].as_str().unwrap().contains("vendor/dep")
    }));
}

#[test]
fn references_keep_ambiguous_package_constructor_leaf_empty() {
    let workspace = TempWorkspace::new("references-ambiguous-package-constructor");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use model from \"example/dep\"\n\n",
            "fn ambiguous() -> model::Left\n",
            "  model::Ready(1)\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"model.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/model.veln",
        concat!(
            "pub type Left\n",
            "  pub Ready(Int)\n",
            "end\n\n",
            "pub type Right\n",
            "  pub Ready(Int)\n",
            "end\n",
        ),
    );

    let result = references_result(&workspace, "main.veln", 4, 10);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["references"],
        json!([]),
        "{result:#}"
    );
}

#[test]
fn references_keep_package_constructor_alias_boundary_empty() {
    let dependency_workspace = TempWorkspace::new("references-dependency-constructor-alias");
    dependency_workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    dependency_workspace.write(
        "main.veln",
        concat!(
            "use model from \"example/dep\"\n\n",
            "fn make() -> model::Alias\n",
            "  model::Alias::Ready(1)\n",
            "end\n",
        ),
    );
    dependency_workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"model.veln\"]\n",
    );
    dependency_workspace.write(
        "vendor/dep/model.veln",
        "pub type Item\n  pub Ready(Int)\nend\n\npub type Alias = Item\n",
    );
    let mut dependency_server = initialized_server(&dependency_workspace);

    let dependency_definition =
        dependency_server.definition_tool(&json!({"source":"main.veln","line":4,"column":17}));
    assert_eq!(
        dependency_definition["isError"], false,
        "{dependency_definition:#}"
    );
    assert_eq!(
        dependency_definition["structuredContent"]["definition"]["range"],
        json!({"start":{"line":2,"column":7},"end":{"line":2,"column":12}})
    );
    let dependency_references =
        dependency_server.references_tool(&json!({"source":"main.veln","line":4,"column":17}));
    assert_eq!(
        dependency_references["isError"], false,
        "{dependency_references:#}"
    );
    assert_eq!(
        dependency_references["structuredContent"]["references"],
        json!([]),
        "{dependency_references:#}"
    );

    let standard_workspace = TempWorkspace::new("references-standard-constructor-alias");
    standard_workspace.write(
        "main.veln",
        "fn make() -> prelude::Alias\n  prelude::Alias::Some(1)\nend\n",
    );
    let mut standard_server = initialized_server(&standard_workspace);
    standard_server
        .language_resources
        .replace_test_standard_library(
            "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
            [PackageSnapshotSource::new(
                "prelude.veln",
                b"pub type Option\n  pub Some(Int)\nend\n\npub type Alias = Option\n",
            )],
        );

    let standard_definition =
        standard_server.definition_tool(&json!({"source":"main.veln","line":2,"column":19}));
    assert_eq!(
        standard_definition["isError"], false,
        "{standard_definition:#}"
    );
    assert_eq!(
        standard_definition["structuredContent"]["definition"]["range"],
        json!({"start":{"line":2,"column":7},"end":{"line":2,"column":11}})
    );
    let standard_references =
        standard_server.references_tool(&json!({"source":"main.veln","line":2,"column":19}));
    assert_eq!(
        standard_references["isError"], false,
        "{standard_references:#}"
    );
    assert_eq!(
        standard_references["structuredContent"]["references"],
        json!([]),
        "{standard_references:#}"
    );
}

#[test]
fn references_return_direct_dependency_function_alias_locations_from_saved_project() {
    let alias_workspace = TempWorkspace::new("references-dependency-alias");
    alias_workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    alias_workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "pub fn first() -> Int\n",
            "  dep::renamed()\n",
            "end\n\n",
            "pub fn second() -> Int\n",
            "  let callback: fn() -> Int = dep::renamed\n",
            "  callback() + dep::renamed() + dep::target()\n",
            "end\n",
        ),
    );
    alias_workspace.write(
        "other.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "pub fn other() -> Int\n",
            "  dep::renamed()\n",
            "end\n",
        ),
    );
    alias_workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    alias_workspace.write(
        "vendor/dep/dep.veln",
        concat!(
            "pub fn target() -> Int\n",
            "  1\n",
            "end\n\n",
            "pub fn renamed = target\n",
        ),
    );
    let mut alias_server = initialized_server(&alias_workspace);
    crate::language_resources::reset_dependency_snapshot_captures();

    let alias_definition =
        alias_server.definition_tool(&json!({"source":"main.veln","line":4,"column":8}));
    assert_eq!(alias_definition["isError"], false, "{alias_definition:#}");
    let alias_uri = alias_definition["structuredContent"]["definition"]["uri"]
        .as_str()
        .unwrap();
    assert!(alias_uri.starts_with("veln-pkg:///example%2Fdep/snapshot/"));
    assert!(alias_uri.ends_with("/dep.veln"));
    assert_eq!(
        alias_definition["structuredContent"]["definition"]["range"],
        json!({"start":{"line":5,"column":8},"end":{"line":5,"column":15}})
    );
    let alias_references =
        alias_server.references_tool(&json!({"source":"main.veln","line":4,"column":8}));
    assert_eq!(alias_references["isError"], false, "{alias_references:#}");
    assert_reference_ranges(
        &alias_references,
        &[
            ("main.veln", 4, 8, 4, 15),
            ("main.veln", 8, 36, 8, 43),
            ("main.veln", 9, 21, 9, 28),
            ("other.veln", 4, 8, 4, 15),
        ],
        "dependency function alias references",
    );
    let references = alias_references["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert!(references.iter().all(|reference| {
        reference["uri"].as_str().unwrap().starts_with("file://")
            && !reference["uri"].as_str().unwrap().contains("veln-pkg:")
            && !reference["uri"].as_str().unwrap().contains("vendor/dep")
    }));

    let target_references =
        alias_server.references_tool(&json!({"source":"main.veln","line":9,"column":38}));
    assert_eq!(target_references["isError"], false, "{target_references:#}");
    assert_reference_ranges(
        &target_references,
        &[("main.veln", 9, 38, 9, 44)],
        "dependency function target references",
    );
    assert_eq!(crate::language_resources::dependency_snapshot_captures(), 1);
}

#[test]
fn references_keep_direct_dependency_function_alias_chains_empty() {
    let alias_workspace = TempWorkspace::new("references-dependency-alias-chain");
    alias_workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    alias_workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "pub fn main() -> Int\n",
            "  dep::chained()\n",
            "end\n",
        ),
    );
    alias_workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    alias_workspace.write(
        "vendor/dep/dep.veln",
        concat!(
            "pub fn target() -> Int\n",
            "  1\n",
            "end\n\n",
            "pub fn renamed = target\n",
            "pub fn chained = renamed\n",
        ),
    );
    let mut alias_server = initialized_server(&alias_workspace);

    let alias_references =
        alias_server.references_tool(&json!({"source":"main.veln","line":4,"column":8}));
    assert_eq!(alias_references["isError"], false, "{alias_references:#}");
    assert_eq!(
        alias_references["structuredContent"]["references"],
        json!([]),
        "{alias_references:#}"
    );
}

#[test]
fn references_keep_invalid_direct_dependency_function_alias_targets_empty() {
    let alias_workspace = TempWorkspace::new("references-dependency-invalid-alias-targets");
    alias_workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    alias_workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "pub fn main() -> Int\n",
            "  dep::missing()\n",
            "  dep::wrong_kind()\n",
            "  dep::invalid_case()\n",
            "end\n",
        ),
    );
    alias_workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    alias_workspace.write(
        "vendor/dep/dep.veln",
        concat!(
            "pub type Document\n",
            "  pub Text(String)\n",
            "end\n\n",
            "pub fn missing = missing\n",
            "pub fn wrong_kind = Document\n",
            "pub fn invalid_case = Missing\n",
        ),
    );
    let mut alias_server = initialized_server(&alias_workspace);

    for (case, line) in [
        ("unresolved target", 4),
        ("wrong-kind target", 5),
        ("invalid-casing target", 6),
    ] {
        let alias_references =
            alias_server.references_tool(&json!({"source":"main.veln","line":line,"column":8}));
        assert_eq!(
            alias_references["isError"], false,
            "{case}: {alias_references:#}"
        );
        assert_eq!(
            alias_references["structuredContent"]["references"],
            json!([]),
            "{case}: {alias_references:#}"
        );
    }
}

#[test]
fn references_keep_invalid_direct_dependency_type_alias_targets_empty() {
    let alias_workspace = TempWorkspace::new("references-dependency-type-alias-invalid-targets");
    alias_workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    alias_workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "pub fn main(input: dep::Missing, wrong: dep::WrongKind, chain: dep::Chained) -> dep::InvalidCase\n",
            "  input\n",
            "end\n",
        ),
    );
    alias_workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    alias_workspace.write(
        "vendor/dep/dep.veln",
        concat!(
            "pub type Item\n",
            "end\n\n",
            "pub schema Packet\n",
            "  format binary\n",
            "  value: UInt8\n",
            "end\n\n",
            "pub type Alias = Item\n",
            "pub type Missing = MissingTarget\n",
            "pub type WrongKind = Packet\n",
            "pub type Chained = Alias\n",
            "pub type InvalidCase = missing_type\n",
        ),
    );
    let mut alias_server = initialized_server(&alias_workspace);

    for (case, column) in [
        ("unresolved target", 25),
        ("wrong-kind target", 45),
        ("alias chain", 67),
        ("invalid-casing target", 87),
    ] {
        let alias_references =
            alias_server.references_tool(&json!({"source":"main.veln","line":3,"column":column}));
        assert_eq!(
            alias_references["isError"], false,
            "{case}: {alias_references:#}"
        );
        assert_eq!(
            alias_references["structuredContent"]["references"],
            json!([]),
            "{case}: {alias_references:#}"
        );
    }
}

#[test]
fn references_keep_direct_dependency_function_aliases_inside_selected_project() {
    struct Case {
        name: &'static str,
        files: Vec<(&'static str, &'static str)>,
        source: &'static str,
        scope: Value,
    }

    let source = concat!(
        "use dep from \"example/dep\"\n\n",
        "fn main() -> Int\n",
        "  dep::renamed()\n",
        "end\n",
    );
    let dependency_manifest =
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n";
    let dependency_source = "pub fn target() -> Int\n  1\nend\n\npub fn renamed = target\n";

    for case in [
        Case {
            name: "anonymous source",
            files: vec![("loose.veln", source)],
            source: "loose.veln",
            scope: json!({
                "mode": "single_file",
                "generation": 0,
                "project": ".",
                "source": "loose.veln",
                "project_wide": false
            }),
        },
        Case {
            name: "descendant project source",
            files: vec![
                ("veln.toml", ""),
                ("main.veln", source),
                ("nested/veln.toml", ""),
                ("nested/main.veln", source),
                ("vendor/dep/veln.toml", dependency_manifest),
                ("vendor/dep/dep.veln", dependency_source),
            ],
            source: "nested/main.veln",
            scope: json!({
                "mode": "single_file",
                "generation": 0,
                "project": ".",
                "source": "nested/main.veln",
                "project_wide": false
            }),
        },
        Case {
            name: "outside selected project",
            files: vec![
                ("app/veln.toml", ""),
                ("app/main.veln", source),
                ("loose.veln", source),
                ("app/vendor/dep/veln.toml", dependency_manifest),
                ("app/vendor/dep/dep.veln", dependency_source),
            ],
            source: "loose.veln",
            scope: json!({
                "mode": "single_file",
                "generation": 0,
                "project": ".",
                "source": "loose.veln",
                "project_wide": false
            }),
        },
    ] {
        let workspace = TempWorkspace::new(case.name);
        for (path, text) in case.files {
            workspace.write(path, text);
        }

        let result = references_result(&workspace, case.source, 4, 8);

        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_eq!(
            result["structuredContent"]["scope"], case.scope,
            "{}: {result:#}",
            case.name
        );
        assert_eq!(
            result["structuredContent"]["references"],
            json!([]),
            "{}: {result:#}",
            case.name
        );
    }
}

#[test]
fn references_keep_direct_dependency_type_aliases_inside_selected_project() {
    struct Case {
        name: &'static str,
        files: Vec<(&'static str, &'static str)>,
        source: &'static str,
        scope: Value,
    }

    let source = concat!(
        "use dep from \"example/dep\"\n\n",
        "fn main(input: dep::Alias) -> dep::Alias\n",
        "  input\n",
        "end\n",
    );
    let dependency_manifest =
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n";
    let dependency_source = "pub type Item\nend\n\npub type Alias = Item\n";

    for case in [
        Case {
            name: "anonymous source",
            files: vec![("loose.veln", source)],
            source: "loose.veln",
            scope: json!({
                "mode": "single_file",
                "generation": 0,
                "project": ".",
                "source": "loose.veln",
                "project_wide": false
            }),
        },
        Case {
            name: "descendant project source",
            files: vec![
                ("veln.toml", ""),
                ("main.veln", source),
                ("nested/veln.toml", ""),
                ("nested/main.veln", source),
                ("vendor/dep/veln.toml", dependency_manifest),
                ("vendor/dep/dep.veln", dependency_source),
            ],
            source: "nested/main.veln",
            scope: json!({
                "mode": "single_file",
                "generation": 0,
                "project": ".",
                "source": "nested/main.veln",
                "project_wide": false
            }),
        },
        Case {
            name: "outside selected project",
            files: vec![
                ("app/veln.toml", ""),
                ("app/main.veln", source),
                ("loose.veln", source),
                ("app/vendor/dep/veln.toml", dependency_manifest),
                ("app/vendor/dep/dep.veln", dependency_source),
            ],
            source: "loose.veln",
            scope: json!({
                "mode": "single_file",
                "generation": 0,
                "project": ".",
                "source": "loose.veln",
                "project_wide": false
            }),
        },
    ] {
        let workspace = TempWorkspace::new(case.name);
        for (path, text) in case.files {
            workspace.write(path, text);
        }

        let result = references_result(&workspace, case.source, 3, 21);

        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_eq!(
            result["structuredContent"]["scope"], case.scope,
            "{}: {result:#}",
            case.name
        );
        assert_eq!(
            result["structuredContent"]["references"],
            json!([]),
            "{}: {result:#}",
            case.name
        );
    }
}

#[test]
fn references_return_standard_library_function_locations() {
    let std_workspace = TempWorkspace::new("references-standard-library-boundary");
    std_workspace.write("veln.toml", "");
    std_workspace.write(
        "main.veln",
        concat!(
            "use math from \"std\"\n\n",
            "fn first(value: Int) -> Int\n",
            "  math::exported(value)\n",
            "end\n\n",
            "fn second(value: Int) -> Int\n",
            "  let callback: fn(Int) -> Int = math::exported\n",
            "  callback(math::exported(value))\n",
            "end\n",
        ),
    );
    let mut std_server = initialized_server(&std_workspace);
    std_server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"math.veln\"]\n",
        [PackageSnapshotSource::new(
            "math.veln",
            b"pub fn exported(value: Int) -> Int\n  value\nend\n",
        )],
    );

    let std_definition =
        std_server.definition_tool(&json!({"source":"main.veln","line":4,"column":9}));
    assert_eq!(std_definition["isError"], false, "{std_definition:#}");
    let std_uri = std_definition["structuredContent"]["definition"]["uri"]
        .as_str()
        .unwrap_or_else(|| {
            panic!("standard library definition should resolve: {std_definition:#}")
        });
    assert!(std_uri.starts_with("veln-pkg:///std/snapshot/"));
    assert!(std_uri.ends_with("/math.veln"));
    let std_references =
        std_server.references_tool(&json!({"source":"main.veln","line":4,"column":9}));
    assert_eq!(std_references["isError"], false, "{std_references:#}");
    assert_reference_ranges(
        &std_references,
        &[
            ("main.veln", 4, 9, 4, 17),
            ("main.veln", 8, 40, 8, 48),
            ("main.veln", 9, 18, 9, 26),
        ],
        "standard library function",
    );
}

#[test]
fn references_keep_standard_library_function_collision_boundaries() {
    let workspace = TempWorkspace::new("references-standard-library-collisions");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use math from \"std\"\n",
            "use dep from \"example/dep\"\n\n",
            "# exported mention\n",
            "pub fn exported(value: Int) -> Int\n",
            "  value\n",
            "end\n\n",
            "pub fn first(record: {exported: Int}, value: Int) -> Int\n",
            "  math::exported(value)\n",
            "  dep::exported(value)\n",
            "  record.exported\n",
            "  \"exported\"\n",
            "  value\n",
            "end\n\n",
            "pub fn second(value: Int) -> Int\n",
            "  let exported = value\n",
            "  let callback: fn(Int) -> Int = math::exported\n",
            "  callback(math::exported(exported))\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/dep.veln",
        "pub fn exported(value: Int) -> Int\n  value\nend\n",
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"math.veln\"]\n",
        [PackageSnapshotSource::new(
            "math.veln",
            concat!(
                "pub fn exported(value: Int) -> Int\n",
                "  exported(value - 1)\n",
                "end\n",
            )
            .as_bytes(),
        )],
    );

    let result = server.references_tool(&json!({"source":"main.veln","line":10,"column":9}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"]["project_wide"], true,
        "{result:#}"
    );
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 10, 9, 10, 17),
            ("main.veln", 19, 40, 19, 48),
            ("main.veln", 20, 18, 20, 26),
        ],
        "standard library collision boundary",
    );

    let alias_segment = server.references_tool(&json!({"source":"main.veln","line":1,"column":5}));
    assert_eq!(alias_segment["isError"], false, "{alias_segment:#}");
    assert_eq!(
        alias_segment["structuredContent"]["references"],
        json!([]),
        "{alias_segment:#}"
    );
}

#[test]
fn references_return_standard_library_type_locations() {
    let workspace = TempWorkspace::new("references-standard-library-type");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "fn first(items: Vec<Int>) -> Vec<Int>\n",
            "  items\n",
            "end\n\n",
            "fn second(items: prelude::Vec<Int>) -> prelude::Vec<Int>\n",
            "  prelude::Vec::Empty()\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        "fn third(items: Vec<Int>) -> prelude::Vec<Int>\n  items\nend\n",
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
        [PackageSnapshotSource::new(
            "prelude.veln",
            b"pub type Vec\n  pub Empty\nend\n",
        )],
    );

    let result = server.references_tool(&json!({"source":"main.veln","line":1,"column":17}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 1, 17, 1, 20),
            ("main.veln", 1, 30, 1, 33),
            ("main.veln", 5, 27, 5, 30),
            ("main.veln", 5, 49, 5, 52),
            ("main.veln", 6, 12, 6, 15),
            ("other.veln", 1, 17, 1, 20),
            ("other.veln", 1, 39, 1, 42),
        ],
        "standard library type",
    );
}

#[test]
fn references_return_standard_library_constructor_locations() {
    let workspace = TempWorkspace::new("references-standard-library-constructor");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "fn first(input: Option<Int>) -> Int\n",
            "  match input\n",
            "    Some(value) => value\n",
            "    None => 0\n",
            "  end\n",
            "end\n\n",
            "fn second() -> Option<Int>\n",
            "  prelude::Option::Some(1)\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        "fn third() -> Option<Int>\n  prelude::Some(2)\nend\n",
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
        [PackageSnapshotSource::new(
            "prelude.veln",
            b"pub type Option\n  pub Some(Int)\n  pub None\nend\n",
        )],
    );

    let result = server.references_tool(&json!({"source":"main.veln","line":3,"column":6}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 3, 5, 3, 9),
            ("main.veln", 9, 20, 9, 24),
            ("other.veln", 2, 12, 2, 16),
        ],
        "standard library constructor",
    );
}

#[test]
fn references_keep_anonymous_sources_isolated_for_new_symbol_classes() {
    let workspace = TempWorkspace::new("references-anonymous-type-isolation");
    workspace.write("app/veln.toml", "");
    workspace.write(
        "app/main.veln",
        "type Item\nend\n\nfn selected(input: Item) -> Item\n  input\nend\n",
    );
    workspace.write(
        "loose.veln",
        "type Item\nend\n\nfn helper(input: Item) -> Item\n  input\nend\n",
    );
    workspace.write(
        "other.veln",
        "type Item\nend\n\nfn helper(input: Item) -> Item\n  input\nend\n",
    );

    let result = references_result(&workspace, "loose.veln", 4, 19);
    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "single_file",
            "generation": 0,
            "project": ".",
            "source": "loose.veln",
            "project_wide": false
        })
    );
    assert_reference_ranges(
        &result,
        &[("loose.veln", 4, 18, 4, 22), ("loose.veln", 4, 27, 4, 31)],
        "anonymous type isolation",
    );
}

#[test]
fn references_keep_descendant_package_sources_isolated_for_new_symbol_classes() {
    let workspace = TempWorkspace::new("references-descendant-package-isolation");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "type Item\nend\n\nfn selected(input: Item) -> Item\n  input\nend\n",
    );
    workspace.write("nested/veln.toml", "");
    workspace.write(
        "nested/main.veln",
        "type Item\nend\n\nfn helper(input: Item) -> Item\n  input\nend\n",
    );

    let result = references_result(&workspace, "nested/main.veln", 4, 19);
    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "single_file",
            "generation": 0,
            "project": ".",
            "source": "nested/main.veln",
            "project_wide": false
        })
    );
    assert_reference_ranges(
        &result,
        &[
            ("nested/main.veln", 4, 18, 4, 22),
            ("nested/main.veln", 4, 27, 4, 31),
        ],
        "descendant package type isolation",
    );
}

#[test]
fn references_keep_anonymous_sources_isolated_for_workspace_schema_selections() {
    let workspace = TempWorkspace::new("references-anonymous-schema-isolation");
    workspace.write("app/veln.toml", "");
    workspace.write(
        "app/main.veln",
        concat!(
            "schema Packet\n",
            "  value: Int\n",
            "end\n\n",
            "fn selected(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode Packet from view at byte_offset(0)?\n",
            "  encode Packet from packet\n",
            "end\n",
        ),
    );
    workspace.write(
        "loose.veln",
        concat!(
            "schema Packet\n",
            "  value: Int\n",
            "end\n\n",
            "fn helper(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode Packet from view at byte_offset(0)?\n",
            "  encode Packet from packet\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        concat!(
            "schema Packet\n",
            "  value: Int\n",
            "end\n\n",
            "fn helper(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode Packet from view at byte_offset(0)?\n",
            "  encode Packet from packet\n",
            "end\n",
        ),
    );

    let result = references_result(&workspace, "loose.veln", 1, 8);
    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "single_file",
            "generation": 0,
            "project": ".",
            "source": "loose.veln",
            "project_wide": false
        })
    );
    assert_reference_ranges(
        &result,
        &[("loose.veln", 6, 10, 6, 16), ("loose.veln", 7, 10, 7, 16)],
        "anonymous schema isolation",
    );
}

#[test]
fn references_keep_descendant_package_sources_isolated_for_workspace_schema_selections() {
    let workspace = TempWorkspace::new("references-descendant-package-schema-isolation");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  value: Int\n",
            "end\n\n",
            "fn selected(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode Packet from view at byte_offset(0)?\n",
            "  encode Packet from packet\n",
            "end\n",
        ),
    );
    workspace.write("nested/veln.toml", "");
    workspace.write(
        "nested/main.veln",
        concat!(
            "schema Packet\n",
            "  value: Int\n",
            "end\n\n",
            "fn helper(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode Packet from view at byte_offset(0)?\n",
            "  encode Packet from packet\n",
            "end\n",
        ),
    );

    let result = references_result(&workspace, "nested/main.veln", 1, 8);
    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "single_file",
            "generation": 0,
            "project": ".",
            "source": "nested/main.veln",
            "project_wide": false
        })
    );
    assert_reference_ranges(
        &result,
        &[
            ("nested/main.veln", 6, 10, 6, 16),
            ("nested/main.veln", 7, 10, 7, 16),
        ],
        "descendant package schema isolation",
    );
}

#[test]
fn references_exclude_declarations_for_new_workspace_symbol_classes() {
    struct Case {
        name: &'static str,
        line: usize,
        column: usize,
        ranges: Vec<(&'static str, usize, usize, usize, usize)>,
    }

    let workspace = TempWorkspace::new("references-declaration-exclusion");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "type Item\n",
            "  Some(Int)\n",
            "end\n\n",
            "fn read(input: Item) -> output: Int\n",
            "  ensure output >= input\n",
            "  let current = input\n",
            "  current\n",
            "end\n\n",
            "fn make(value: Int) -> Item\n",
            "  Some(value)\n",
            "end\n\n",
            "effect Run\n",
            "  call(action: fn() -> Int) -> Int\n",
            "end\n\n",
            "handler run(callback: fn(Int) -> Int) handles Run\n",
            "  call(action) => callback(action())\n",
            "end\n",
        ),
    );

    let cases = [
        Case {
            name: "type declaration",
            line: 1,
            column: 6,
            ranges: vec![("main.veln", 5, 16, 5, 20), ("main.veln", 11, 24, 11, 28)],
        },
        Case {
            name: "constructor use",
            line: 12,
            column: 4,
            ranges: vec![("main.veln", 12, 3, 12, 7)],
        },
        Case {
            name: "function parameter use",
            line: 6,
            column: 21,
            ranges: vec![("main.veln", 6, 20, 6, 25), ("main.veln", 7, 17, 7, 22)],
        },
        Case {
            name: "result binding use",
            line: 6,
            column: 11,
            ranges: vec![("main.veln", 6, 10, 6, 16)],
        },
        Case {
            name: "handler context parameter use",
            line: 19,
            column: 20,
            ranges: vec![("main.veln", 20, 19, 20, 27)],
        },
        Case {
            name: "handler operation clause parameter use",
            line: 20,
            column: 29,
            ranges: vec![("main.veln", 20, 28, 20, 34)],
        },
    ];

    for case in cases {
        let result = references_result(&workspace, "main.veln", case.line, case.column);
        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_reference_ranges(&result, &case.ranges, case.name);
    }
}

#[test]
fn references_reject_recovery_package_and_unsupported_symbols() {
    struct Case {
        name: &'static str,
        files: Vec<(&'static str, &'static str)>,
        source: &'static str,
        line: usize,
        column: usize,
    }

    let cases = [
        Case {
            name: "recovery value binding",
            files: vec![
                ("veln.toml", ""),
                ("main.veln", "fn main(Bad: Int) -> Int\n  Bad\nend\n"),
            ],
            source: "main.veln",
            line: 2,
            column: 4,
        },
        Case {
            name: "package private type",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use example::dep\n\nfn read(input: dep::Item) -> dep::Item\n  input\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                ("vendor/dep/dep.veln", "type Item\nend\n"),
            ],
            source: "main.veln",
            line: 3,
            column: 25,
        },
        Case {
            name: "package non-exported type",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use hidden from \"example/dep\"\n\nfn read(input: hidden::Item) -> hidden::Item\n  input\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"public.veln\"]\n",
                ),
                ("vendor/dep/public.veln", "pub fn ok() -> Int\n  1\nend\n"),
                ("vendor/dep/hidden.veln", "pub type Item\nend\n"),
            ],
            source: "main.veln",
            line: 3,
            column: 28,
        },
        Case {
            name: "package invalid-casing type",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use dep from \"example/dep\"\n\nfn read(input: dep::item) -> dep::item\n  input\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                ("vendor/dep/dep.veln", "pub type item\nend\n"),
            ],
            source: "main.veln",
            line: 3,
            column: 25,
        },
        Case {
            name: "workspace public schema alias",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "pub schema Packet\n",
                        "  format binary\n",
                        "  value: UInt8\n",
                        "end\n\n",
                        "pub schema AliasPacket = Packet\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 12,
        },
        Case {
            name: "workspace schema composition target",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "pub schema Packet\n",
                        "  format binary\n",
                        "  value: UInt8\n",
                        "end\n\n",
                        "schema Frame\n",
                        "  format binary\n",
                        "  nested: Packet\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 8,
            column: 11,
        },
        Case {
            name: "workspace schema decode qualifier",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "pub schema Packet\n",
                        "  format binary\n",
                        "  value: UInt8\n",
                        "end\n",
                    ),
                ),
                (
                    "other.veln",
                    concat!(
                        "use main\n\n",
                        "fn imported(view: ByteView) -> ()\n",
                        "  decode main::Packet from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            source: "other.veln",
            line: 4,
            column: 12,
        },
        Case {
            name: "workspace schema encode qualifier",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "pub schema Packet\n",
                        "  format binary\n",
                        "  value: UInt8\n",
                        "end\n",
                    ),
                ),
                (
                    "other.veln",
                    concat!(
                        "use main\n\n",
                        "fn imported(packet: {value: Int}) -> ()\n",
                        "  encode main::Packet from packet\n",
                        "end\n",
                    ),
                ),
            ],
            source: "other.veln",
            line: 4,
            column: 12,
        },
        Case {
            name: "private package constructor",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use dep from \"example/dep\"\n\nfn make() -> dep::Item\n  dep::Item::Ready(1)\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                ("vendor/dep/dep.veln", "pub type Item\n  Ready(Int)\nend\n"),
            ],
            source: "main.veln",
            line: 4,
            column: 15,
        },
        Case {
            name: "package schema",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use dep from \"example/dep\"\n\nfn read(view: ByteView) -> ()\n  decode dep::Packet from view at byte_offset(0)?\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                (
                    "vendor/dep/dep.veln",
                    "pub schema Packet\n  value: Int\nend\n",
                ),
            ],
            source: "main.veln",
            line: 4,
            column: 15,
        },
        Case {
            name: "effect operation",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "effect Task\n  run() -> Int\nend\n\nfn main() -> Int effects [Task]\n  perform Task::run()\nend\n",
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 17,
        },
        Case {
            name: "effect",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "effect Task\n  run() -> Int\nend\n\nfn main() -> Int effects [Task]\n  1\nend\n",
                ),
            ],
            source: "main.veln",
            line: 5,
            column: 32,
        },
        Case {
            name: "handler",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "effect Task\n  run() -> Int\nend\n\nhandler task() handles Task\n  run() => 1\nend\n\nfn main() -> Int effects [Task]\n  handle perform Task::run() with task()\nend\n",
                ),
            ],
            source: "main.veln",
            line: 10,
            column: 34,
        },
        Case {
            name: "casing neutral type selection",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "type item\n  Value\nend\n\nfn read(input: item) -> item\n  input\nend\n",
                ),
            ],
            source: "main.veln",
            line: 5,
            column: 17,
        },
        Case {
            name: "no symbol",
            files: vec![("main.veln", "fn main() -> Int\n  1\nend\n")],
            source: "main.veln",
            line: 2,
            column: 3,
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        for (path, text) in case.files {
            workspace.write(path, text);
        }
        let result = references_result(&workspace, case.source, case.line, case.column);
        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_eq!(
            result["structuredContent"]["references"],
            json!([]),
            "{}: {result:#}",
            case.name
        );
    }
}

#[test]
fn references_preserve_unicode_coordinates_and_token_end_exclusion() {
    let workspace = TempWorkspace::new("references-coordinate-boundaries");
    workspace.write(
        "main.veln",
        "fn main() -> Int\r\n  let emoji = \"🙂\"\r\n  main()\r\nend\r\n",
    );

    let selected = references_result(&workspace, "main.veln", 3, 4);
    assert_eq!(selected["isError"], false, "{selected:#}");
    assert_reference_ranges(
        &selected,
        &[("main.veln", 3, 3, 3, 7)],
        "unicode coordinate selection",
    );

    let token_end = references_result(&workspace, "main.veln", 3, 7);
    assert_eq!(token_end["isError"], false, "{token_end:#}");
    assert_eq!(token_end["structuredContent"]["references"], json!([]));

    let invalid = references_result(&workspace, "main.veln", 2, 21);
    assert_eq!(invalid["isError"], true, "{invalid:#}");
    assert_eq!(invalid["structuredContent"]["code"], "invalid_position");
}

#[test]
fn references_use_single_file_scope_for_sources_outside_selected_projects() {
    let workspace = TempWorkspace::new("references-single-file");
    workspace.write("app/veln.toml", "");
    workspace.write("app/main.veln", "fn selected() -> Int\n  selected()\nend\n");
    workspace.write("loose.veln", "fn helper() -> Int\n  helper()\nend\n");
    workspace.write("other.veln", "fn helper() -> Int\n  helper()\nend\n");

    let result = references_result(&workspace, "loose.veln", 2, 4);
    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "single_file",
            "generation": 0,
            "project": ".",
            "source": "loose.veln",
            "project_wide": false
        })
    );
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert_eq!(references.len(), 1, "{result:#}");
    assert!(
        references[0]["uri"]
            .as_str()
            .unwrap()
            .ends_with("loose.veln"),
        "{result:#}"
    );
    assert_eq!(
        references[0]["range"],
        json!({"start": {"line": 2, "column": 3}, "end": {"line": 2, "column": 9}})
    );
}

#[test]
fn references_do_not_expose_function_shaped_recovery_records() {
    let workspace = TempWorkspace::new("references-recovery-boundary");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "test Bad() -> Int\n",
            "  Bad()\n",
            "end\n\n",
            "fn read() -> Int\n",
            "  Bad()\n",
            "end\n",
        ),
    );

    let result = references_result(&workspace, "main.veln", 6, 4);
    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(result["structuredContent"]["references"], json!([]));
    assert_eq!(result["structuredContent"]["scope"]["project_wide"], true);
}

#[test]
fn references_report_invalid_positions_and_schema_coordinate_failures() {
    let workspace = TempWorkspace::new("references-invalid-position");
    workspace.write("main.veln", "fn main() -> Int\n  main()\nend\n");

    let invalid = references_result(&workspace, "main.veln", 5, 1);
    assert_eq!(invalid["isError"], true, "{invalid:#}");
    assert_eq!(invalid["structuredContent"]["code"], "invalid_position");
    assert!(
        invalid["structuredContent"]
            .as_object()
            .unwrap()
            .get("references")
            .is_none()
    );

    let non_integer_request = serde_json::from_str(
        r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "references",
                "arguments": {
                    "source": "main.veln",
                    "line": 2.0000000000000001,
                    "column": 4
                }
            }
        }"#,
    )
    .unwrap();
    let non_integer = initialized_server(&workspace)
        .handle_request(non_integer_request)
        .unwrap();
    assert_eq!(non_integer["error"]["code"], -32602, "{non_integer:#}");
    assert!(non_integer.get("result").is_none(), "{non_integer:#}");
}

#[test]
fn references_reject_paths_and_changed_workspace_identity() {
    let workspace = TempWorkspace::new("references-boundaries");
    workspace.write("main.veln", "fn main() -> Int\n  main()\nend\n");
    for source in ["../main.veln", "missing.veln", "main.txt"] {
        let result = references_result(&workspace, source, 1, 1);
        assert_eq!(
            result["structuredContent"]["code"], "invalid_path",
            "{source}"
        );
    }

    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();
    let selection = Selection::discover(base.path()).unwrap();
    fs::remove_dir_all(&workspace.root).unwrap();
    workspace.write("main.veln", "fn main() -> Int\n  main()\nend\n");
    let mut server = Server {
        base,
        selection,
        initialized: true,
        language_resources: minimal_language_resources(),
    };
    let result = server.references_tool(&json!({"source":"main.veln","line":2,"column":4}));
    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
    assert!(
        result["structuredContent"]
            .as_object()
            .unwrap()
            .get("references")
            .is_none()
    );
}

#[test]
fn references_project_capture_exhausts_retries_after_owned_source_changes() {
    let workspace = TempWorkspace::new("references-project-capture-retry");
    write_workspace_with_dependency_and_sources(
        &workspace,
        "fn main() -> Int\n  value()\nend\n\nfn value() -> Int\n  1\nend\n",
        Some("fn helper() -> Int\n  1\nend\n"),
    );
    let mut server = initialized_server(&workspace);
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let main = root.join("main.veln");
        fs::remove_file(&main).unwrap();
        if attempt % 2 == 0 {
            fs::write(
                &main,
                "fn main() -> Int\n  value()\nend\n\nfn value() -> Int\n  2\nend\n",
            )
            .unwrap();
            fs::remove_file(root.join("helper.veln")).unwrap();
        } else {
            fs::write(
                &main,
                "fn main() -> Int\n  value()\nend\n\nfn value() -> Int\n  1\nend\n",
            )
            .unwrap();
            fs::write(root.join("helper.veln"), "fn helper() -> Int\n  1\nend\n").unwrap();
        }
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":2,"column":4}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}

#[test]
fn references_project_capture_exhausts_retries_for_workspace_schema_selection() {
    let workspace = TempWorkspace::new("references-workspace-schema-capture-retry");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  value: Int\n",
            "end\n\n",
            "fn read(view: ByteView) -> ()\n",
            "  decode Packet from view at byte_offset(0)?\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let main = root.join("main.veln");
        fs::remove_file(&main).unwrap();
        let field = if attempt % 2 == 0 { "value" } else { "other" };
        fs::write(
            &main,
            format!(
                "schema Packet\n  {field}: Int\nend\n\nfn read(view: ByteView) -> ()\n  decode Packet from view at byte_offset(0)?\nend\n"
            ),
        )
        .unwrap();
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":1,"column":8}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
}

#[test]
fn references_project_capture_exhausts_retries_after_dependency_source_changes() {
    let workspace = TempWorkspace::new("references-dependency-capture-retry");
    write_workspace_with_dependency_and_sources(
        &workspace,
        "use dep from \"example/dep\"\n\nfn main() -> Int\n  dep::value()\nend\n",
        None,
    );
    let mut server = initialized_server(&workspace);
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let source = root.join("vendor/dep/dep.veln");
        fs::remove_file(&source).unwrap();
        let value = if attempt % 2 == 0 { 2 } else { 1 };
        fs::write(&source, format!("pub fn value() -> Int\n  {value}\nend\n")).unwrap();
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":4,"column":9}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}

#[test]
fn references_project_capture_exhausts_retries_for_dependency_function_alias_selection() {
    let workspace = TempWorkspace::new("references-dependency-alias-capture-retry");
    write_workspace_with_dependency_and_sources(
        &workspace,
        "use dep from \"example/dep\"\n\nfn main() -> Int\n  dep::renamed()\nend\n",
        None,
    );
    workspace.write(
        "vendor/dep/dep.veln",
        "pub fn value() -> Int\n  1\nend\n\npub fn renamed = value\n",
    );
    let mut server = initialized_server(&workspace);
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let source = root.join("main.veln");
        fs::remove_file(&source).unwrap();
        let value = if attempt % 2 == 0 { 1 } else { 2 };
        fs::write(
            &source,
            format!("use dep from \"example/dep\"\n\nfn main() -> Int\n  dep::renamed() + {value}\nend\n"),
        )
        .unwrap();
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":4,"column":9}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}

#[test]
fn references_project_capture_exhausts_retries_for_dependency_type_alias_selection() {
    let workspace = TempWorkspace::new("references-dependency-type-alias-capture-retry");
    write_workspace_with_dependency_and_sources(
        &workspace,
        "use dep from \"example/dep\"\n\nfn main(input: dep::Alias) -> dep::Alias\n  input\nend\n",
        None,
    );
    workspace.write(
        "vendor/dep/dep.veln",
        "pub type Item\nend\n\npub type Alias = Item\n",
    );
    let mut server = initialized_server(&workspace);
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let source = root.join("main.veln");
        fs::remove_file(&source).unwrap();
        let value = if attempt % 2 == 0 { 1 } else { 2 };
        fs::write(
            &source,
            format!("use dep from \"example/dep\"\n\nfn main(input: dep::Alias) -> dep::Alias\n  let value: Int = {value}\n  input\nend\n"),
        )
        .unwrap();
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":3,"column":21}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}

#[test]
fn references_project_capture_exhausts_retries_for_dependency_type_selection() {
    let workspace = TempWorkspace::new("references-dependency-type-capture-retry");
    write_workspace_with_dependency_and_sources(
        &workspace,
        "use dep from \"example/dep\"\n\nfn main(input: dep::Item) -> dep::Item\n  input\nend\n",
        None,
    );
    workspace.write("vendor/dep/dep.veln", "pub type Item\nend\n");
    let mut server = initialized_server(&workspace);
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let source = root.join("vendor/dep/dep.veln");
        fs::remove_file(&source).unwrap();
        let body = if attempt % 2 == 0 {
            "pub type Item\n  pub Ready(Int)\nend\n"
        } else {
            "pub type Item\nend\n"
        };
        fs::write(&source, body).unwrap();
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":3,"column":21}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}

#[test]
fn references_project_capture_exhausts_retries_for_dependency_constructor_selection() {
    let workspace = TempWorkspace::new("references-dependency-constructor-capture-retry");
    write_workspace_with_dependency_and_sources(
        &workspace,
        "use dep from \"example/dep\"\n\nfn main() -> dep::Item\n  dep::Item::Ready(1)\nend\n",
        None,
    );
    workspace.write(
        "vendor/dep/dep.veln",
        "pub type Item\n  pub Ready(Int)\nend\n",
    );
    let mut server = initialized_server(&workspace);
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let source = root.join("vendor/dep/dep.veln");
        fs::remove_file(&source).unwrap();
        let body = if attempt % 2 == 0 {
            "pub type Item\n  pub Ready(Int)\n  pub Other(Int)\nend\n"
        } else {
            "pub type Item\n  pub Ready(Int)\nend\n"
        };
        fs::write(&source, body).unwrap();
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":4,"column":15}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}

#[test]
fn references_project_capture_exhausts_retries_for_standard_library_selection() {
    let workspace = TempWorkspace::new("references-standard-library-capture-retry");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "use math from \"std\"\n\nfn main() -> Int\n  math::value()\nend\n",
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"math.veln\"]\n",
        [PackageSnapshotSource::new(
            "math.veln",
            b"pub fn value() -> Int\n  1\nend\n",
        )],
    );
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let source = root.join("main.veln");
        fs::remove_file(&source).unwrap();
        let value = if attempt % 2 == 0 { 1 } else { 2 };
        fs::write(
            &source,
            format!("use math from \"std\"\n\nfn main() -> Int\n  math::value() + {value}\nend\n"),
        )
        .unwrap();
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":4,"column":9}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
}

#[test]
fn references_anonymous_capture_exhausts_retries_after_requested_source_changes() {
    let workspace = TempWorkspace::new("references-anonymous-capture-retry");
    workspace.write(
        "loose.veln",
        "fn main() -> Int\n  value()\nend\n\nfn value() -> Int\n  1\nend\n",
    );
    let mut server = initialized_server(&workspace);
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let source = root.join("loose.veln");
        fs::remove_file(&source).unwrap();
        let value = if attempt % 2 == 0 { 2 } else { 1 };
        fs::write(
            &source,
            format!("fn main() -> Int\n  value()\nend\n\nfn value() -> Int\n  {value}\nend\n"),
        )
        .unwrap();
    });

    let result = server.references_tool(&json!({"source":"loose.veln","line":2,"column":4}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
}

fn references_result(workspace: &TempWorkspace, source: &str, line: usize, column: usize) -> Value {
    initialized_server(workspace)
        .references_tool(&json!({"source": source, "line": line, "column": column}))
}

fn assert_snapshot_changed_without_references_or_scope(result: &Value) {
    assert_eq!(result["isError"], true, "{result:#}");
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
    let structured = result["structuredContent"].as_object().unwrap();
    assert!(!structured.contains_key("references"), "{result:#}");
    assert!(!structured.contains_key("scope"), "{result:#}");
}

fn all_resource_state(server: &mut Server) -> Value {
    let resources = server
        .handle_request(json!({"jsonrpc":"2.0","id":"references-state","method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .clone();
    json!(resources)
}

fn dependency_resource_is_listed(server: &mut Server, identity: &str) -> bool {
    let prefix = format!("veln-pkg:///{}/snapshot/", identity.replace('/', "%2F"));
    all_resource_state(server)
        .as_array()
        .unwrap()
        .iter()
        .any(|resource| resource["uri"].as_str().unwrap().starts_with(&prefix))
}

fn write_workspace_with_dependency_and_sources(
    workspace: &TempWorkspace,
    main: &str,
    helper: Option<&str>,
) {
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write("main.veln", main);
    if let Some(helper) = helper {
        workspace.write("helper.veln", helper);
    }
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    workspace.write("vendor/dep/dep.veln", "pub fn value() -> Int\n  1\nend\n");
}

fn write_dependency_reference_workspace(
    workspace: &TempWorkspace,
    field: &str,
    source: &str,
    selector: Option<&str>,
) {
    let selector = selector
        .map(|selector| format!("{selector}\n"))
        .unwrap_or_default();
    workspace.write(
        "veln.toml",
        &format!("[dependencies.\"example/dep\"]\n{field} = \"{source}\"\n{selector}"),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use lib::math from \"example/dep\"\n\n",
            "pub fn first(value: Int) -> Int\n",
            "  math::increase(value)\n",
            "end\n\n",
            "pub fn second(value: Int) -> Int\n",
            "  let callback: fn(Int) -> Int = math::increase\n",
            "  callback(math::increase(value))\n",
            "end\n",
        ),
    );
    write_dependency_package(workspace, source, "example/dep", "lib/math.veln");
}

fn write_dependency_package(workspace: &TempWorkspace, root: &str, identity: &str, export: &str) {
    workspace.write(
        &format!("{root}/veln.toml"),
        &format!("[package]\nname = \"{identity}\"\n\n[lib]\nexports = [\"{export}\"]\n"),
    );
    workspace.write(
        &format!("{root}/{export}"),
        concat!(
            "pub fn increase(value: Int) -> Int\n",
            "  increase(value - 1)\n",
            "end\n",
            "\n",
            "pub fn target(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
        ),
    );
}

fn assert_reference_ranges(
    result: &Value,
    expected: &[(&str, usize, usize, usize, usize)],
    name: &str,
) {
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap_or_else(|| panic!("{name}: references must be an array: {result:#}"));
    assert_eq!(
        references.len(),
        expected.len(),
        "{name}: unexpected reference count: {result:#}"
    );
    for (reference, (file, start_line, start_column, end_line, end_column)) in
        references.iter().zip(expected)
    {
        let uri = reference["uri"].as_str().unwrap();
        assert!(
            uri.starts_with("file://"),
            "{name}: expected canonical file URI: {reference:#}"
        );
        assert!(
            !uri.contains("/./") && !uri.contains("/../"),
            "{name}: expected normalized file URI: {reference:#}"
        );
        assert!(uri.ends_with(file), "{name}: {reference:#}");
        assert_eq!(
            reference["range"],
            json!({
                "start": {"line": start_line, "column": start_column},
                "end": {"line": end_line, "column": end_column}
            }),
            "{name}: {reference:#}"
        );
    }
}
