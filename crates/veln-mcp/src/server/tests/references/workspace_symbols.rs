use super::*;

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

    let with_declaration = initialized_server(&workspace).references_tool(&json!({
        "source": "main.veln",
        "line": 10,
        "column": 4,
        "include_declaration": true
    }));
    assert_reference_ranges(
        &with_declaration,
        &[
            ("main.veln", 5, 4, 5, 10),
            ("main.veln", 6, 3, 6, 9),
            ("main.veln", 10, 3, 10, 9),
        ],
        "workspace function declaration inclusion",
    );

    let omitted = references_result(&workspace, "main.veln", 10, 4);
    let explicit_false = initialized_server(&workspace).references_tool(&json!({
        "source": "main.veln",
        "line": 10,
        "column": 4,
        "include_declaration": false
    }));
    assert_eq!(omitted, explicit_false);
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
fn references_return_workspace_schema_composition_locations_and_scope() {
    let workspace = TempWorkspace::new("references-workspace-schema-composition");
    workspace.write("veln.toml", "");
    workspace.write(
        "app/wire.veln",
        concat!(
            "pub schema Packet\n",
            "  format binary\n",
            "  value: UInt8\n",
            "end\n\n",
            "schema LocalFrame\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  direct: Packet\n",
            "  repeated: Repeat(count, Packet)\n",
            "  canonical: [Packet; count]\n",
            "end\n\n",
            "pub schema Collision\n",
            "  format binary\n",
            "  value: UInt8\n",
            "end\n\n",
            "pub type Collision\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        concat!(
            "use app::wire\n\n",
            "schema ImportedFrame\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  qualified: app::wire::Packet\n",
            "  alias_qualified: wire::Packet\n",
            "  repeated: Repeat(count, app::wire::Packet)\n",
            "  canonical: [wire::Packet; count]\n",
            "  bare: Packet\n",
            "  unresolved: missing::Packet\n",
            "  collision: wire::Collision\n",
            "end\n\n",
            "fn lexical_noise() -> String\n",
            "  # app::wire::Packet wire::Packet Packet\n",
            "  \"app::wire::Packet wire::Packet Packet\"\n",
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
            ("app/wire.veln", 9, 11, 9, 17),
            ("app/wire.veln", 10, 27, 10, 33),
            ("app/wire.veln", 11, 15, 11, 21),
            ("other.veln", 6, 25, 6, 31),
            ("other.veln", 7, 26, 7, 32),
            ("other.veln", 8, 38, 8, 44),
            ("other.veln", 9, 21, 9, 27),
        ],
        "workspace schema composition references",
    );

    for (name, source, line, column) in [
        ("ordinary type collision", "app/wire.veln", 14, 12),
        ("unresolved composition path", "other.veln", 11, 24),
        ("composition lexical noise", "other.veln", 16, 16),
    ] {
        let unsupported = references_result(&workspace, source, line, column);
        assert_eq!(unsupported["isError"], false, "{name}: {unsupported:#}");
        assert_reference_ranges(&unsupported, &[], name);
    }
}

#[test]
fn references_keep_same_named_workspace_schema_composition_identity() {
    let workspace = TempWorkspace::new("references-workspace-schema-composition-identity");
    workspace.write("veln.toml", "");
    workspace.write(
        "first.veln",
        "pub schema Packet\n  format binary\n  value: UInt8\nend\n",
    );
    workspace.write(
        "second.veln",
        "pub schema Packet\n  format binary\n  value: UInt8\nend\n",
    );
    workspace.write(
        "host.veln",
        concat!(
            "use first\n",
            "use second\n\n",
            "schema Host\n",
            "  format binary\n",
            "  first_packet: first::Packet\n",
            "  second_packet: second::Packet\n",
            "end\n",
        ),
    );

    let first = references_result(&workspace, "first.veln", 1, 12);
    assert_eq!(first["isError"], false, "{first:#}");
    assert_reference_ranges(
        &first,
        &[("host.veln", 6, 24, 6, 30)],
        "first same-named workspace schema composition references",
    );

    let second = references_result(&workspace, "second.veln", 1, 12);
    assert_eq!(second["isError"], false, "{second:#}");
    assert_reference_ranges(
        &second,
        &[("host.veln", 7, 26, 7, 32)],
        "second same-named workspace schema composition references",
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
                (
                    "main.veln",
                    "schema PrivatePacket\n  format binary\n  value: UInt8\nend\n",
                ),
                (
                    "main.test.veln",
                    concat!(
                        "use main\n\n",
                        "schema CompanionFrame\n",
                        "  format binary\n",
                        "  count: UInt8\n",
                        "  direct: main::PrivatePacket\n",
                        "  repeated: [main::PrivatePacket; count]\n",
                        "end\n\n",
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
                        "schema UnrelatedFrame\n",
                        "  format binary\n",
                        "  direct: main::PrivatePacket\n",
                        "end\n\n",
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
                ("main.test.veln", 6, 17, 6, 30),
                ("main.test.veln", 7, 20, 7, 33),
                ("main.test.veln", 11, 30, 11, 43),
                ("main.test.veln", 12, 30, 12, 43),
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
