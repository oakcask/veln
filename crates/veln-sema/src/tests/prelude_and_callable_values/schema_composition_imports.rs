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

#[test]
fn invalid_explicit_alias_recovers_only_a_unique_public_schema_target() {
    for target in [
        concat!(
            "pub schema Packet\n",
            "  format binary\n",
            "  value: UInt8\n",
            "end\n",
        ),
        concat!(
            "pub schema Base\n",
            "  format binary\n",
            "  value: UInt8\n",
            "end\n",
            "pub schema Packet = Base\n",
        ),
    ] {
        let module = merged_modules_with_identities(vec![
            (
                "transport::wire",
                SourceFile::new("transport/wire.veln", target),
            ),
            (
                "main",
                SourceFile::new(
                    "main.veln",
                    concat!(
                        "use transport::wire as Wire\n",
                        "schema Host\n",
                        "  format binary\n",
                        "  nested: Wire::Packet\n",
                        "end\n",
                        "fn decode_packet(view: ByteView, base: ByteOffset) -> DecodeStep<{value: Int}>\n",
                        "  decode Wire::Packet from view at base\n",
                        "end\n",
                    ),
                ),
            ),
        ]);

        let diagnostics = analyze_surface_module(&module);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "name.invalid_case"),
            "{diagnostics:#?}"
        );
        assert!(
            diagnostics.iter().all(|diagnostic| {
                diagnostic.id != "schema.composition_reference"
                    && diagnostic.id != "schema.decode_expression"
            }),
            "{diagnostics:#?}"
        );
        let lowered = lower_checked_surface_module(&module);
        assert!(lowered.core.is_none());
        assert!(lowered.ir.is_none());
    }
}

#[test]
fn invalid_explicit_alias_recovery_rejects_unproved_schema_targets() {
    let cases = [
        (
            "reserved",
            "use transport::wire as prelude\n",
            "nested: prelude::Packet",
            "pub schema Packet\n  format binary\n  value: UInt8\nend\n",
        ),
        (
            "syntax_recovered",
            "use transport::wire as\n",
            "nested: wire::Packet",
            "pub schema Packet\n  format binary\n  value: UInt8\nend\n",
        ),
        (
            "missing",
            "use transport::wire as Wire\n",
            "nested: Wire::Missing",
            "pub schema Packet\n  format binary\n  value: UInt8\nend\n",
        ),
        (
            "private",
            "use transport::wire as Wire\n",
            "nested: Wire::Packet",
            "schema Packet\n  format binary\n  value: UInt8\nend\n",
        ),
        (
            "wrong_kind",
            "use transport::wire as Wire\n",
            "nested: Wire::Packet",
            "pub type Packet\nend\n",
        ),
    ];
    for (case, import, field, target) in cases {
        let module = merged_modules_with_identities(vec![
            (
                "transport::wire",
                SourceFile::new("transport/wire.veln", target),
            ),
            (
                "main",
                SourceFile::new(
                    "main.veln",
                    format!("{import}schema Host\n  format binary\n  {field}\nend\n"),
                ),
            ),
        ]);
        let diagnostics = analyze_surface_module(&module);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "schema.composition_reference"),
            "{case}: {diagnostics:#?}"
        );
    }

    let colliding = merged_modules_with_identities(vec![
        (
            "transport::wire",
            SourceFile::new(
                "transport/wire.veln",
                "pub schema Packet\n  format binary\n  value: UInt8\nend\n",
            ),
        ),
        (
            "other::wire",
            SourceFile::new(
                "other/wire.veln",
                "pub schema Packet\n  format binary\n  value: UInt8\nend\n",
            ),
        ),
        (
            "main",
            SourceFile::new(
                "main.veln",
                concat!(
                    "use transport::wire as Wire\n",
                    "use other::wire as Wire\n",
                    "schema Host\n",
                    "  format binary\n",
                    "  nested: Wire::Packet\n",
                    "end\n",
                ),
            ),
        ),
    ]);
    assert!(
        analyze_surface_module(&colliding)
            .iter()
            .any(|diagnostic| diagnostic.id == "schema.composition_reference")
    );

    let ambiguous = merged_modules_with_identities(vec![
        (
            "transport::wire",
            SourceFile::new(
                "transport/first.veln",
                "pub schema Packet\n  format binary\n  value: UInt8\nend\n",
            ),
        ),
        (
            "transport::wire",
            SourceFile::new(
                "transport/second.veln",
                "pub schema Packet\n  format binary\n  value: UInt8\nend\n",
            ),
        ),
        (
            "main",
            SourceFile::new(
                "main.veln",
                concat!(
                    "use transport::wire as Wire\n",
                    "schema Host\n",
                    "  format binary\n",
                    "  nested: Wire::Packet\n",
                    "end\n",
                ),
            ),
        ),
    ]);
    assert!(
        analyze_surface_module(&ambiguous)
            .iter()
            .any(|diagnostic| diagnostic.id == "schema.composition_reference")
    );
}

#[test]
fn schema_composition_import_lookup_work_is_bounded_by_imports_and_references() {
    for (import_count, reference_count) in [(32usize, 16usize), (64, 16), (64, 32)] {
        let mut source = String::from("mod main\n");
        for index in 0..import_count {
            source.push_str(&format!("use dependency{index}\n"));
        }
        let module = lower_surface_ast(&parse(&SourceFile::new("main.veln", source)).tree);

        crate::name_recovery::reset_schema_composition_import_lookup_work();
        for index in 0..reference_count {
            let import = format!("dependency{}", index % import_count);
            assert!(
                crate::name_recovery::schema_composition_imported_use_for_path(
                    &module,
                    &[import],
                    Some("main"),
                )
                .is_some()
            );
        }
        let work = crate::name_recovery::take_schema_composition_import_lookup_work();
        assert_eq!(work.lookups, reference_count, "{work:?}");
        assert_eq!(
            work.indexed_imports,
            import_count * reference_count,
            "{work:?}"
        );
        assert_eq!(
            work.quarantine_checks,
            import_count * reference_count,
            "collision quarantine must reuse the per-reference index: {work:?}"
        );
    }
}
