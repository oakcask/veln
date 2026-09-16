use super::*;

#[test]
fn schema_composition_resolves_workspace_import_leaf_aliases_before_collision_checks() {
    let module = merged_modules_with_identities(vec![
        (
            "app::wire",
            SourceFile::new(
                "app/wire.veln",
                concat!(
                    "pub schema Packet\n",
                    "  format binary\n",
                    "  value: UInt8\n",
                    "end\n",
                    "pub schema Collision\n",
                    "  format binary\n",
                    "  value: UInt8\n",
                    "end\n",
                    "pub type Collision\n",
                    "end\n",
                ),
            ),
        ),
        (
            "main",
            SourceFile::new(
                "main.veln",
                concat!(
                    "use app::wire\n",
                    "schema Host\n",
                    "  format binary\n",
                    "  count: UInt8\n",
                    "  direct: wire::Packet\n",
                    "  repeated: [wire::Packet; count]\n",
                    "  collision: wire::Collision\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);
    let composition_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "schema.composition_reference")
        .collect::<Vec<_>>();

    assert_eq!(composition_diagnostics.len(), 1, "{diagnostics:#?}");
    assert!(matches!(
        &composition_diagnostics[0].details,
        veln_diagnostics::JsonValue::Object(entries)
            if entries.iter().any(|(key, value)| {
                key == "binding"
                    && value == &veln_diagnostics::JsonValue::string("collision")
            }) && entries.iter().any(|(key, value)| {
                key == "reason"
                    && value
                        == &veln_diagnostics::JsonValue::string("ambiguous_type_and_schema")
            })
    ));
}

#[test]
fn schema_composition_prefers_exact_workspace_import_over_package_leaf_alias() {
    for imports in [
        "use wire\nuse a::wire from \"example/pkg\"\n",
        "use a::wire from \"example/pkg\"\nuse wire\n",
    ] {
        let module = merged_modules_with_identities(vec![
            (
                "wire",
                SourceFile::new("wire.veln", "pub schema Packet\n  value: Int\nend\n"),
            ),
            (
                "a::wire",
                SourceFile::new("a/wire.veln", "pub schema Packet\n  value: String\nend\n"),
            ),
            (
                "main",
                SourceFile::new(
                    "main.veln",
                    format!("{imports}\nschema Host\n  nested: wire::Packet\nend\n"),
                ),
            ),
        ]);

        let references = resolved_schema_composition_references(&module);

        assert_eq!(references.len(), 1, "{references:#?}");
        assert_eq!(references[0].target_span.file.as_str(), "wire.veln");
    }
}
