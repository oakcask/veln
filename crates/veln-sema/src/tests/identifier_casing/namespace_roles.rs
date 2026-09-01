use super::*;

#[test]
fn uppercase_namespaces_resolve_by_use_role() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Item\n",
            "  Item(value: Int)\n",
            "end\n",
            "\n",
            "schema Item\n",
            "  format binary\n",
            "  value: UInt8\n",
            "end\n",
            "\n",
            "effect Item\n",
            "  Item(value: Int) -> Int\n",
            "end\n",
            "\n",
            "handler Item(offset: Int) handles Item\n",
            "  Item(value) => value + offset\n",
            "end\n",
            "\n",
            "fn from_constructor(value: Int) -> Item\n",
            "  Item(value)\n",
            "end\n",
            "\n",
            "fn from_schema() -> Result<ByteChunk, EncodeError>\n",
            "  encode Item from {value: 7}\n",
            "end\n",
            "\n",
            "fn from_effect(value: Int) -> Int effects [Item]\n",
            "  perform Item::Item(value)\n",
            "end\n",
            "\n",
            "fn from_handler(value: Int) -> Int\n",
            "  handle from_effect(value) with Item(1)\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert!(lowered.core.is_some());
    assert!(lowered.ir.is_some());
}

#[test]
fn lowercase_namespaces_resolve_by_use_role_and_value_shadowing() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema item\n",
            "  value: Int\n",
            "end\n",
            "\n",
            "effect item\n",
            "  item(value: Int) -> Int\n",
            "end\n",
            "\n",
            "handler item(offset: Int) handles item\n",
            "  item(value) => value + offset\n",
            "end\n",
            "\n",
            "fn item(value: Int) -> Int\n",
            "  value + 10\n",
            "end\n",
            "\n",
            "fn from_schema(value: {value: Int}) -> Result<{value: Int}, String>\n",
            "  encode item from value\n",
            "end\n",
            "\n",
            "fn from_effect(value: Int) -> Int effects [item]\n",
            "  perform item::item(value)\n",
            "end\n",
            "\n",
            "fn main(callback: fn(Int) -> Int, value: Int) -> Int\n",
            "  let item = callback\n",
            "  handle item(value) with item(1)\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert!(lowered.core.is_some());
    assert!(lowered.ir.is_some());
}

#[test]
fn ordinary_calls_exclude_casing_neutral_namespaces() {
    for (name, source_text) in [
        (
            "uppercase",
            concat!(
                "schema Item\n",
                "  value: Int\n",
                "end\n",
                "\n",
                "effect Item\n",
                "  Item() -> Int\n",
                "end\n",
                "\n",
                "handler Item() handles Item\n",
                "  Item() => 1\n",
                "end\n",
                "\n",
                "fn main() -> Int effects [Item]\n",
                "  Item()\n",
                "end\n",
            ),
        ),
        (
            "lowercase",
            concat!(
                "schema item\n",
                "  value: Int\n",
                "end\n",
                "\n",
                "effect item\n",
                "  item() -> Int\n",
                "end\n",
                "\n",
                "handler item() handles item\n",
                "  item() => 1\n",
                "end\n",
                "\n",
                "fn main() -> Int effects [item]\n",
                "  item()\n",
                "end\n",
            ),
        ),
    ] {
        let module = lower_surface_ast(&parse(&SourceFile::new("main.veln", source_text)).tree);
        let diagnostics = analyze_surface_module(&module);

        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "name.unresolved"
                    && diagnostic.message.contains("unresolved call_target")
            }),
            "{name}: {diagnostics:#?}"
        );
    }
}

#[test]
fn same_namespace_duplicates_remain_kind_local() {
    let cases = [
        (
            "type",
            "type Same\n  First\nend\ntype Same\n  Second\nend\n",
            "duplicate type declaration name `Same`",
        ),
        (
            "constructor",
            "type Owner\n  Same\n  Same\nend\n",
            "duplicate constructor declaration name `Same`",
        ),
        (
            "function",
            "fn same() -> Int\n  1\nend\nfn same() -> Int\n  2\nend\n",
            "duplicate function declaration name `same`",
        ),
        (
            "schema",
            "schema Same\n  value: Int\nend\nschema Same\n  other: Int\nend\n",
            "duplicate schema declaration name `Same`",
        ),
        (
            "effect",
            "effect Same\nend\neffect Same\nend\n",
            "duplicate effect declaration name `Same`",
        ),
        (
            "handler",
            "effect Same\nend\nhandler Same() handles Same\nend\nhandler Same() handles Same\nend\n",
            "duplicate handler declaration name `Same`",
        ),
        (
            "operation",
            "effect Same\n  value() -> Int\n  value() -> Int\nend\n",
            "duplicate effect operation declaration name `value`",
        ),
    ];

    for (name, source_text, message) in cases {
        let module = lower_surface_ast(&parse(&SourceFile::new("main.veln", source_text)).tree);
        let diagnostics = analyze_surface_module(&module);

        assert!(
            diagnostics.iter().any(
                |diagnostic| diagnostic.id == "name.duplicate" && diagnostic.message == message
            ),
            "{name}: {diagnostics:#?}"
        );
    }
}

#[test]
fn schema_composition_preserves_type_schema_ambiguity() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Shared\n",
            "  Shared(value: Int)\n",
            "end\n",
            "\n",
            "schema Shared\n",
            "  value: Int\n",
            "end\n",
            "\n",
            "schema Host\n",
            "  child: Shared\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "schema.composition_reference"
                && diagnostic
                    .details
                    .to_json()
                    .contains("\"reason\":\"ambiguous_type_and_schema\"")
        }),
        "{:#?}",
        lowered.diagnostics
    );
}
