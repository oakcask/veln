use super::*;

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
        WorkspaceSymbolCase {
            name: "begin local captured by deferred cleanup",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "fn cleanup(value: Int) -> ()\n",
                        "  ()\n",
                        "end\n\n",
                        "fn read(input: Int) -> Int\n",
                        "  begin\n",
                        "    let captured = input\n",
                        "    defer\n",
                        "      cleanup(captured)\n",
                        "    end\n",
                        "    captured\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 9,
            column: 15,
            ranges: vec![("main.veln", 9, 15, 9, 23), ("main.veln", 11, 5, 11, 13)],
        },
        WorkspaceSymbolCase {
            name: "parameter shadowed across nested cleanup scopes",
            files: nested_cleanup_shadowing_files(),
            source: "main.veln",
            line: 2,
            column: 15,
            ranges: vec![("main.veln", 2, 15, 2, 20)],
        },
        WorkspaceSymbolCase {
            name: "outer local shadowed across nested cleanup scopes",
            files: nested_cleanup_shadowing_files(),
            source: "main.veln",
            line: 11,
            column: 4,
            ranges: vec![("main.veln", 4, 17, 4, 22), ("main.veln", 11, 3, 11, 8)],
        },
        WorkspaceSymbolCase {
            name: "begin local shadowed by deferred cleanup local",
            files: nested_cleanup_shadowing_files(),
            source: "main.veln",
            line: 9,
            column: 6,
            ranges: vec![("main.veln", 6, 19, 6, 24), ("main.veln", 9, 5, 9, 10)],
        },
        WorkspaceSymbolCase {
            name: "deferred cleanup innermost local",
            files: nested_cleanup_shadowing_files(),
            source: "main.veln",
            line: 7,
            column: 8,
            ranges: vec![("main.veln", 7, 7, 7, 12)],
        },
        WorkspaceSymbolCase {
            name: "cleanup region type annotations in functions and handlers",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "type Resource\n",
                        "  Ready\n",
                        "end\n\n",
                        "effect Ask\n",
                        "  value() -> Resource\n",
                        "end\n\n",
                        "fn work(input: Resource) -> Resource\n",
                        "  defer\n",
                        "    let deferred: Resource = input\n",
                        "    ()\n",
                        "  end\n",
                        "  let begun: Resource = begin\n",
                        "    let nested: Resource = input\n",
                        "    nested\n",
                        "  end\n",
                        "  begun\n",
                        "end\n\n",
                        "handler ask(seed: Resource) handles Ask\n",
                        "  value() => begin\n",
                        "    defer\n",
                        "      let deferred: Resource = seed\n",
                        "      ()\n",
                        "    end\n",
                        "    let clause: Resource = seed\n",
                        "    clause\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 27,
            column: 19,
            ranges: vec![
                ("main.veln", 6, 14, 6, 22),
                ("main.veln", 9, 16, 9, 24),
                ("main.veln", 9, 29, 9, 37),
                ("main.veln", 11, 19, 11, 27),
                ("main.veln", 14, 14, 14, 22),
                ("main.veln", 15, 17, 15, 25),
                ("main.veln", 21, 19, 21, 27),
                ("main.veln", 24, 21, 24, 29),
                ("main.veln", 27, 17, 27, 25),
            ],
        },
    ];

    assert_workspace_symbol_cases(cases);
}

fn nested_cleanup_shadowing_files() -> Vec<(&'static str, &'static str)> {
    vec![
        ("veln.toml", ""),
        (
            "main.veln",
            concat!(
                "fn read(value: Int) -> Int\n",
                "  let value = value\n",
                "  begin\n",
                "    let value = value\n",
                "    defer\n",
                "      let value = value\n",
                "      value\n",
                "    end\n",
                "    value\n",
                "  end\n",
                "  value\n",
                "end\n",
            ),
        ),
    ]
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
