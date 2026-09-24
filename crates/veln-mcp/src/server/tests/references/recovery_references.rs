use super::*;

struct RecoveryCase {
    name: &'static str,
    source: &'static str,
    declaration: (usize, usize),
    reference: (usize, usize),
    ranges: &'static [(usize, usize, usize)],
}

#[test]
fn references_expose_every_shared_recovery_identity_from_declaration_and_use() {
    let cases = [
        RecoveryCase {
            name: "type",
            source: "type item\nend\n\nfn read(value: item) -> item\n  value\nend\n",
            declaration: (1, 6),
            reference: (4, 17),
            ranges: &[(4, 16, 20), (4, 25, 29)],
        },
        RecoveryCase {
            name: "constructor",
            source: "type Item\n  value(input: Int)\nend\n\nfn read() -> Item\n  value(1)\nend\n",
            declaration: (2, 3),
            reference: (6, 4),
            ranges: &[(6, 3, 8)],
        },
        RecoveryCase {
            name: "function",
            source: "fn Bad() -> Int\n  Bad()\nend\n\nfn read() -> Int\n  Bad()\nend\n",
            declaration: (1, 4),
            reference: (6, 4),
            ranges: &[(2, 3, 6), (6, 3, 6)],
        },
        RecoveryCase {
            name: "test",
            source: "test Bad() -> Int\n  Bad()\nend\n\nfn read() -> Int\n  Bad()\nend\n",
            declaration: (1, 6),
            reference: (6, 4),
            ranges: &[(2, 3, 6), (6, 3, 6)],
        },
        RecoveryCase {
            name: "function parameter",
            source: "fn main(Bad: Int) -> Int\n  Bad\nend\n",
            declaration: (1, 9),
            reference: (2, 4),
            ranges: &[(2, 3, 6)],
        },
        RecoveryCase {
            name: "result binding",
            source: "fn main(value: Int) -> Output: Int\n  ensure Output >= value\n  value\nend\n",
            declaration: (1, 24),
            reference: (2, 11),
            ranges: &[(2, 10, 16)],
        },
        RecoveryCase {
            name: "local binding",
            source: "fn main(input: Int) -> Int\n  let Bad = input\n  Bad\nend\n",
            declaration: (2, 7),
            reference: (3, 4),
            ranges: &[(3, 3, 6)],
        },
        RecoveryCase {
            name: "pattern binding",
            source: "type Item\n  Some(Int)\nend\n\nfn read(item: Item) -> Int\n  match item\n    Some(Bad) => Bad\n  end\nend\n",
            declaration: (7, 10),
            reference: (7, 19),
            ranges: &[(7, 18, 21)],
        },
        RecoveryCase {
            name: "satisfy candidate",
            source: "fn main(limit: Int) -> Int\n  _value satisfy Candidate => Candidate <= limit\n  limit\nend\n",
            declaration: (2, 18),
            reference: (2, 32),
            ranges: &[(2, 31, 40)],
        },
        RecoveryCase {
            name: "handler context parameter",
            source: "effect Adjust\n  amount(value: Int) -> Int\nend\n\nhandler adjust(Callback: fn(Int) -> Int) handles Adjust\n  amount(value) => Callback(value)\nend\n",
            declaration: (5, 16),
            reference: (6, 22),
            ranges: &[(6, 20, 28)],
        },
        RecoveryCase {
            name: "handler operation-clause parameter",
            source: "effect Adjust\n  amount(value: Int) -> Int\nend\n\nhandler adjust() handles Adjust\n  amount(Result) => Result\nend\n",
            declaration: (6, 10),
            reference: (6, 21),
            ranges: &[(6, 21, 27)],
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        workspace.write("veln.toml", "");
        workspace.write("main.veln", case.source);
        let expected = case
            .ranges
            .iter()
            .map(|(line, start, end)| ("main.veln", *line, *start, *line, *end))
            .collect::<Vec<_>>();

        let from_reference =
            references_result(&workspace, "main.veln", case.reference.0, case.reference.1);
        let from_declaration = references_result(
            &workspace,
            "main.veln",
            case.declaration.0,
            case.declaration.1,
        );
        assert_reference_ranges(&from_reference, &expected, case.name);
        assert_eq!(
            from_declaration["structuredContent"]["references"],
            from_reference["structuredContent"]["references"],
            "{}",
            case.name
        );

        let with_declaration = initialized_server(&workspace).references_tool(&json!({
            "source": "main.veln",
            "line": case.reference.0,
            "column": case.reference.1,
            "include_declaration": true
        }));
        assert_reference_ranges(
            &with_declaration,
            &std::iter::once((
                "main.veln",
                case.declaration.0,
                case.declaration.1,
                case.declaration.0,
                case.declaration.1 + declaration_width(case.source, case.declaration),
            ))
            .chain(expected.iter().copied())
            .collect::<Vec<_>>(),
            case.name,
        );
    }
}

fn declaration_width(source: &str, (line, column): (usize, usize)) -> usize {
    source
        .lines()
        .nth(line - 1)
        .unwrap()
        .chars()
        .skip(column - 1)
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .count()
}

#[test]
fn recovery_reference_declaration_policy_and_pagination_keep_existing_contract() {
    let workspace = TempWorkspace::new("recovery-reference-pagination");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "fn Bad() -> Int\r\n  Bad(\"🙂\") + Bad()\r\nend\r\n",
    );
    let mut server = initialized_server(&workspace);

    let without_declaration = server.references_tool(&json!({
        "source":"main.veln", "line":2, "column":15,
        "include_declaration":false
    }));
    assert_reference_ranges(
        &without_declaration,
        &[("main.veln", 2, 3, 2, 6), ("main.veln", 2, 14, 2, 17)],
        "recovery declaration exclusion",
    );

    let first = server.references_tool(&json!({
        "source":"main.veln", "line":2, "column":15,
        "include_declaration":true, "page_size":1
    }));
    assert_reference_ranges(&first, &[("main.veln", 1, 4, 1, 7)], "recovery first page");
    let second = server.references_tool(&json!({
        "cursor": first["structuredContent"]["next_cursor"]
    }));
    assert_reference_ranges(
        &second,
        &[("main.veln", 2, 3, 2, 6)],
        "recovery second page",
    );
    let third = server.references_tool(&json!({
        "cursor": second["structuredContent"]["next_cursor"]
    }));
    assert_reference_ranges(
        &third,
        &[("main.veln", 2, 14, 2, 17)],
        "recovery final page",
    );
    assert!(third["structuredContent"].get("next_cursor").is_none());
}

#[test]
fn declaration_only_recovery_succeeds_with_an_empty_or_declaration_result() {
    let workspace = TempWorkspace::new("declaration-only-recovery");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", "fn Bad() -> Int\n  1\nend\n");

    let without = references_result(&workspace, "main.veln", 1, 4);
    assert_reference_ranges(&without, &[], "declaration-only recovery exclusion");
    let with = initialized_server(&workspace).references_tool(&json!({
        "source":"main.veln", "line":1, "column":4,
        "include_declaration":true
    }));
    assert_reference_ranges(
        &with,
        &[("main.veln", 1, 4, 1, 7)],
        "declaration-only recovery inclusion",
    );
}

#[test]
fn recovery_references_keep_shared_unsupported_selection_boundaries() {
    let cases = [
        (
            "ambiguous recovery",
            "fn Bad() -> Int\n  1\nend\n\nfn Bad() -> Int\n  2\nend\n\nfn read() -> Int\n  Bad()\nend\n",
            10,
            4,
        ),
        (
            "qualified recovery occurrence",
            "fn Bad() -> Int\n  1\nend\n\nfn read() -> Int\n  missing::Bad()\nend\n",
            6,
            12,
        ),
        (
            "shadowed recovery binding",
            "fn main(Bad: Int) -> Int\n  match Bad\n    Bad => Bad\n  end\nend\n",
            3,
            12,
        ),
        (
            "recovery binding initializer",
            "fn main(input: Int) -> Int\n  let Bad = Bad\n  Bad\nend\n",
            2,
            14,
        ),
        (
            "outside recovery lexical scope",
            "fn main(input: Int) -> Int\n  let Bad = input\n  Bad\nend\n\nfn other() -> Int\n  Bad\nend\n",
            7,
            4,
        ),
        (
            "class-incompatible recovery occurrence",
            "type item\nend\n\nfn read() -> Int\n  item()\nend\n",
            5,
            4,
        ),
        (
            "symbol class without recovery identity",
            "effect choose\n  pick() -> Int\nend\n",
            1,
            8,
        ),
    ];

    for (name, source, line, column) in cases {
        let workspace = TempWorkspace::new(name);
        workspace.write("veln.toml", "");
        workspace.write("main.veln", source);
        let result = initialized_server(&workspace).references_tool(&json!({
            "source":"main.veln", "line":line, "column":column,
            "include_declaration":true
        }));
        assert_eq!(result["isError"], false, "{name}: {result:#}");
        assert_reference_ranges(&result, &[], name);
    }
}

#[test]
fn recovery_references_exclude_text_only_and_unrelated_same_spelled_occurrences() {
    let workspace = TempWorkspace::new("recovery-reference-occurrence-boundaries");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "fn Bad() -> Int\n",
            "  Bad()\n",
            "end\n\n",
            "fn scoped(Bad: Int) -> Int\n",
            "  # Bad() is only a comment.\n",
            "  let label = \"Bad\"\n",
            "  Bad\n",
            "end\n\n",
            "fn caller() -> Int\n",
            "  Bad()\n",
            "end\n",
        ),
    );

    let function = references_result(&workspace, "main.veln", 12, 4);
    assert_reference_ranges(
        &function,
        &[("main.veln", 2, 3, 2, 6), ("main.veln", 12, 3, 12, 6)],
        "function recovery excludes text-only and unrelated occurrences",
    );
}

#[test]
fn recovery_references_reject_invalid_module_identity() {
    let workspace = TempWorkspace::new("recovery-invalid-module-identity");
    workspace.write("veln.toml", "");
    workspace.write("App/_net.veln", "fn Bad() -> Int\n  Bad()\nend\n");

    let result = initialized_server(&workspace).references_tool(&json!({
        "source":"App/_net.veln", "line":2, "column":4,
        "include_declaration":true
    }));
    assert_eq!(result["isError"], false, "{result:#}");
    assert_reference_ranges(&result, &[], "invalid recovery module identity");
}

#[test]
fn recovery_references_keep_anonymous_sources_isolated() {
    let workspace = TempWorkspace::new("recovery-anonymous-isolation");
    workspace.write("app/veln.toml", "");
    workspace.write("app/main.veln", "fn selected() -> Int\n  selected()\nend\n");
    workspace.write("loose.veln", "fn Bad() -> Int\n  Bad()\nend\n");
    workspace.write("other.veln", "fn Bad() -> Int\n  Bad()\nend\n");

    let result = initialized_server(&workspace).references_tool(&json!({
        "source":"loose.veln", "line":2, "column":4,
        "include_declaration":true
    }));
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode":"single_file", "generation":0, "project":".",
            "source":"loose.veln", "project_wide":false
        })
    );
    assert_reference_ranges(
        &result,
        &[("loose.veln", 1, 4, 1, 7), ("loose.veln", 2, 3, 2, 6)],
        "anonymous recovery isolation",
    );
}
