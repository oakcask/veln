use super::*;

#[test]
fn invalid_target_with_valid_explicit_alias_recovers_the_real_module_across_carriers() {
    for (written_target, actual_target) in
        [("Http", "http"), ("transport::Http", "transport::http")]
    {
        let module = merged_modules_with_identities(vec![
            (
                "app",
                SourceFile::new(
                    "app.veln",
                    format!(
                        concat!(
                            "use {} as wire\n",
                            "pub fn exported = wire::entry\n",
                            "pub type Exported = wire::Payload\n",
                            "pub schema ExportedWire = wire::Wire\n",
                            "fn built() -> wire::Payload\n",
                            "  wire::Payload::Data(wire::entry())\n",
                            "end\n",
                            "fn refined(value: wire::Payload::Data) -> wire::Payload::Data\n",
                            "  value\n",
                            "end\n",
                            "fn inspected(value: wire::Payload) -> Int\n",
                            "  match value\n",
                            "    wire::Payload::Data(inner) => inner\n",
                            "    wire::Payload::Empty => 0\n",
                            "  end\n",
                            "end\n",
                            "schema Packet\n",
                            "  payload: wire::Wire\n",
                            "end\n",
                            "fn effected() -> () effects [wire::Audit]\n",
                            "  ()\n",
                            "end\n",
                            "fn body() -> Int\n",
                            "  1\n",
                            "end\n",
                            "fn handled() -> Int\n",
                            "  handle body() with wire::audit()\n",
                            "end\n",
                        ),
                        written_target,
                    ),
                ),
            ),
            (
                actual_target,
                SourceFile::new(
                    "http.veln",
                    concat!(
                        "pub fn entry() -> Int\n",
                        "  1\n",
                        "end\n",
                        "pub type Payload\n",
                        "  pub Data(Int)\n",
                        "  pub Empty\n",
                        "end\n",
                        "pub schema Wire\n",
                        "  value: Int\n",
                        "end\n",
                        "pub effect Audit\n",
                        "  record() -> Int\n",
                        "end\n",
                        "pub handler audit() for Audit\n",
                        "  record() => 1\n",
                        "end\n",
                    ),
                ),
            ),
        ]);
        let environment = TypeEnvironment::from_module(&module);
        let constructor_path = [
            "wire".to_string(),
            "Payload".to_string(),
            "Data".to_string(),
        ];
        assert_eq!(
            environment.quarantined_import_constructor_recovery_candidate_count(
                &constructor_path,
                Some("app"),
                Some(1),
            ),
            1
        );
        assert_eq!(
            environment
                .quarantined_explicit_import_constructor(&constructor_path, Some("app"))
                .map(|constructor| constructor.descriptor.module_name.as_deref()),
            Some(Some(actual_target))
        );

        let diagnostics = analyze_surface_module(&module);
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.invalid_case")
                .count(),
            1,
            "written_target={written_target}: {diagnostics:#?}"
        );
        for derivative in [
            "name.unresolved",
            "type.mismatch",
            "type.match_non_exhaustive",
            "core.constructor_arity_mismatch",
            "schema.composition_reference",
            "effect.unknown",
            "handler.unknown",
        ] {
            assert!(
                diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.id != derivative),
                "written_target={written_target}, unexpected {derivative}: {diagnostics:#?}"
            );
        }
        assert!(lower_checked_surface_module(&module).core.is_none());
    }
}

#[test]
fn invalid_target_recovery_requires_one_public_callable_and_no_import_collision() {
    for (declaration, unresolved) in [
        ("pub fn entry() -> Int\n  1\nend\n", false),
        ("pub fn other() -> Int\n  1\nend\n", true),
        ("fn entry() -> Int\n  1\nend\n", true),
        ("pub let entry: Int = 1\n", true),
        (
            "pub fn entry() -> Int\n  1\nend\npub fn entry() -> Int\n  2\nend\n",
            true,
        ),
    ] {
        let module = merged_modules_with_identities(vec![
            (
                "app",
                SourceFile::new(
                    "app.veln",
                    "use Http as wire\nfn main() -> Int\n  wire::entry()\nend\n",
                ),
            ),
            ("http", SourceFile::new("http.veln", declaration)),
        ]);
        let diagnostics = analyze_surface_module(&module);
        assert_eq!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "name.unresolved"
                    && diagnostic.message == "unresolved call_target `wire::entry`"
            }),
            unresolved,
            "declaration={declaration:?}: {diagnostics:#?}"
        );
        assert!(lower_checked_surface_module(&module).core.is_none());
    }

    for imports in [
        "use Http as wire\nuse other as wire\n",
        "use Http as wire\nuse Http as second\n",
    ] {
        let module = merged_modules_with_identities(vec![
            (
                "app",
                SourceFile::new(
                    "app.veln",
                    format!("{imports}fn main() -> Int\n  wire::entry()\nend\n"),
                ),
            ),
            (
                "http",
                SourceFile::new("http.veln", "pub fn entry() -> Int\n  1\nend\n"),
            ),
            (
                "other",
                SourceFile::new("other.veln", "pub fn entry() -> Int\n  2\nend\n"),
            ),
        ]);
        let diagnostics = analyze_surface_module(&module);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "name.duplicate"),
            "imports={imports:?}: {diagnostics:#?}"
        );
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved call_target `wire::entry`"
        }));
        assert!(lower_checked_surface_module(&module).core.is_none());
    }
}

#[test]
fn invalid_target_recovery_keeps_constructor_failures_and_both_casing_diagnostics() {
    let module = merged_modules_with_identities(vec![
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use transport::Http as HTTP\n",
                    "fn wrong_arity() -> HTTP::Payload\n",
                    "  HTTP::Payload::Data(1, 2)\n",
                    "end\n",
                    "fn incomplete(value: HTTP::Payload) -> Int\n",
                    "  match value\n",
                    "    HTTP::Payload::Data(inner) => inner\n",
                    "  end\n",
                    "end\n",
                ),
            ),
        ),
        (
            "transport::http",
            SourceFile::new(
                "http.veln",
                concat!(
                    "pub type Payload\n",
                    "  pub Data(Int)\n",
                    "  pub Empty\n",
                    "end\n",
                ),
            ),
        ),
    ]);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.span.as_ref().is_some_and(|span| {
                span.start.line == 1 && span.start.column == 16 && span.end.column == 20
            })
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic
                .details
                .to_json()
                .contains("\"occurrence\":\"declaration\"")
            && diagnostic.span.as_ref().is_some_and(|span| {
                span.start.line == 1 && span.start.column == 24 && span.end.column == 28
            })
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "core.constructor_arity_mismatch"
            && diagnostic
                .details
                .to_json()
                .contains("\"expected_argument_count\":1")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.match_non_exhaustive"
            && diagnostic.message == "match is missing case Empty"
    }));
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.message != "unresolved call_target `HTTP::Payload::Data`"
    }));
    assert!(lower_checked_surface_module(&module).core.is_none());
}
