use super::*;

#[test]
fn explicit_import_alias_casing_uses_the_alias_token_and_quarantines_the_import() {
    let app_source = SourceFile::new(
        "app.veln",
        "mod app\nuse http as HTTP\nfn main() -> Int\n  HTTP::entry()\nend\n",
    );
    let http_source = SourceFile::new("http.veln", "mod http\npub fn entry() -> Int\n  1\nend\n");
    let mut module = lower_surface_ast(&parse(&app_source).tree);
    let http = lower_surface_ast(&parse(&http_source).tree);
    module.functions.extend(http.functions);
    module.invalid_names.extend(http.invalid_names);
    let diagnostics = analyze_surface_module(&module);
    let casing = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "name.invalid_case")
        .expect("alias casing diagnostic");

    assert_eq!(casing.span.as_ref().unwrap().start.column, 13);
    assert!(
        casing
            .details
            .to_json()
            .contains("\"occurrence\":\"declaration\"")
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
    assert!(lower_checked_surface_module(&module).core.is_none());
}

#[test]
fn mixed_invalid_and_valid_explicit_aliases_quarantine_the_repeated_target_in_both_orders() {
    for imports in [
        "use transport::http as HTTP\nuse transport::http as wire\n",
        "use transport::http as wire\nuse transport::http as HTTP\n",
    ] {
        let module = merged_modules(vec![
            SourceFile::new(
                "app.veln",
                format!(
                    "mod app\n{imports}use transport::other as control\n\
                     fn main() -> Int\n\
                       HTTP::entry() + wire::entry() + transport::http::entry() + entry() + transport::other::entry()\n\
                     end\n"
                ),
            ),
            SourceFile::new(
                "http.veln",
                "mod transport::http\npub fn entry() -> Int\n  1\nend\n",
            ),
            SourceFile::new(
                "other.veln",
                "mod transport::other\npub fn entry() -> Int\n  2\nend\n",
            ),
        ]);
        let diagnostics = analyze_surface_module(&module);
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.duplicate")
                .count(),
            1,
            "{diagnostics:#?}"
        );
        for qualifier in [vec!["HTTP"], vec!["wire"], vec!["transport", "http"]] {
            assert!(
                crate::name_recovery::normal_imported_use_for_path(
                    &module,
                    &qualifier
                        .iter()
                        .map(|part| (*part).to_string())
                        .collect::<Vec<_>>(),
                    Some("app"),
                )
                .is_none(),
                "imports={imports:?}, qualifier={qualifier:?}"
            );
        }
        for target in [
            "HTTP::entry",
            "wire::entry",
            "transport::http::entry",
            "entry",
        ] {
            assert!(
                diagnostics.iter().any(|diagnostic| {
                    diagnostic.id == "name.unresolved"
                        && diagnostic.message == format!("unresolved call_target `{target}`")
                }),
                "imports={imports:?}, target={target}, diagnostics={diagnostics:#?}"
            );
        }
        assert_eq!(
            crate::name_recovery::normal_imported_use_for_path(
                &module,
                &["transport".to_string(), "other".to_string()],
                Some("app"),
            )
            .map(|use_decl| use_decl.name.as_str()),
            Some("transport::other")
        );
        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic.message != "unresolved call_target `transport::other::entry`"
        }));
        assert!(lower_checked_surface_module(&module).core.is_none());
    }
}

#[test]
fn invalid_explicit_alias_recovery_requires_one_public_callable_target() {
    for alias in ["HTTP", "_wire", "_"] {
        // Underscore-led expression heads are holes; use the retained exact path
        // to check that these aliases quarantine the same import atomically.
        let qualifier = if alias == "HTTP" {
            alias
        } else {
            "transport::http"
        };
        for (declaration, unresolved) in [
            ("pub fn entry() -> Int\n  1\nend\n", false),
            ("pub fn other() -> Int\n  1\nend\n", true),
            ("fn entry() -> Int\n  1\nend\n", true),
            ("pub let entry: Int = 1\n", true),
        ] {
            let module = merged_modules(vec![
                SourceFile::new(
                    "app.veln",
                    format!(
                        "mod app\nuse transport::http as {alias}\nfn main() -> Int\n  {qualifier}::entry()\nend\n"
                    ),
                ),
                SourceFile::new("http.veln", format!("mod transport::http\n{declaration}")),
            ]);
            let diagnostics = analyze_surface_module(&module);
            let casing = diagnostics
                .iter()
                .find(|diagnostic| diagnostic.id == "name.invalid_case")
                .expect("invalid explicit alias should have a casing diagnostic");
            assert_diagnostic_span(casing, 2, 24, 2, 24 + alias.len());
            let details = casing.details.to_json();
            for field in [
                "\"occurrence\":\"declaration\"",
                "\"name_class\":\"module\"",
                "\"required_initial\":\"ascii_lowercase\"",
            ] {
                assert!(details.contains(field), "{details}");
            }
            let observed = if alias == "HTTP" {
                "ascii_uppercase"
            } else {
                "underscore"
            };
            assert!(details.contains(&format!("\"observed_initial\":\"{observed}\"")));
            assert_eq!(
                diagnostics.iter().any(|diagnostic| {
                    diagnostic.id == "name.unresolved"
                        && diagnostic.message
                            == format!("unresolved call_target `{qualifier}::entry`")
                }),
                unresolved,
                "alias={alias}, declaration={declaration:?}, diagnostics={diagnostics:#?}"
            );
            for qualifier in [vec![alias], vec!["http"], vec!["transport", "http"]] {
                assert!(
                    crate::name_recovery::normal_imported_use_for_path(
                        &module,
                        &qualifier
                            .iter()
                            .map(|part| (*part).to_string())
                            .collect::<Vec<_>>(),
                        Some("app"),
                    )
                    .is_none()
                );
            }
            assert!(lower_checked_surface_module(&module).core.is_none());
        }
    }
}

#[test]
fn invalid_explicit_alias_recovery_suppresses_only_proven_carrier_cascades() {
    let module = merged_modules(vec![
        SourceFile::new(
            "app.veln",
            concat!(
                "mod app\n",
                "use transport::http as HTTP\n",
                "fn typed(value: HTTP::Payload) -> HTTP::Payload\n",
                "  value\n",
                "end\n",
                "fn built() -> HTTP::Payload\n",
                "  HTTP::Payload::Data(1)\n",
                "end\n",
                "fn inspected(value: HTTP::Payload) -> Int\n",
                "  match value\n",
                "    HTTP::Payload::Data(inner) => inner\n",
                "  end\n",
                "end\n",
                "schema Packet\n",
                "  payload: HTTP::Wire\n",
                "end\n",
                "fn effected() -> () effects [HTTP::Audit]\n",
                "  ()\n",
                "end\n",
                "fn body() -> Int\n",
                "  1\n",
                "end\n",
                "fn handled() -> Int\n",
                "  handle body() with HTTP::audit()\n",
                "end\n",
            ),
        ),
        SourceFile::new(
            "http.veln",
            concat!(
                "mod transport::http\n",
                "pub type Payload\n",
                "  pub Data(Int)\n",
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
    ]);
    let environment = TypeEnvironment::from_module(&module);
    let constructor_path = [
        "HTTP".to_string(),
        "Payload".to_string(),
        "Data".to_string(),
    ];
    assert_eq!(
        environment.quarantined_import_constructor_recovery_candidate_count(
            &constructor_path,
            Some("app"),
            None,
        ),
        1
    );
    assert!(
        environment
            .quarantined_explicit_import_constructor(&constructor_path, Some("app"))
            .is_some()
    );
    assert_eq!(
        environment.quarantined_import_constructor_recovery_candidate_count(
            &constructor_path,
            Some("app"),
            Some(2),
        ),
        0
    );
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case")
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
            "unexpected {derivative}: {diagnostics:#?}"
        );
    }
    assert!(
        TypeEnvironment::from_module(&module)
            .uses
            .iter()
            .all(|use_decl| use_decl.alias != "HTTP")
    );
    assert!(lower_checked_surface_module(&module).core.is_none());
}

#[test]
fn invalid_explicit_alias_recovery_preserves_independent_carrier_failures() {
    let module = merged_modules(vec![
        SourceFile::new(
            "app.veln",
            concat!(
                "mod app\n",
                "use transport::http as HTTP\n",
                "pub type MissingAlias = HTTP::Missing\n",
                "fn missing_type(value: HTTP::Missing) -> Int\n",
                "  value\n",
                "end\n",
                "fn wrong_arity() -> HTTP::Payload\n",
                "  HTTP::Payload::Data(1, 2)\n",
                "end\n",
                "fn incomplete(value: HTTP::Payload) -> Int\n",
                "  match value\n",
                "    HTTP::Payload::Data(inner) => inner\n",
                "  end\n",
                "end\n",
                "schema Packet\n",
                "  payload: HTTP::Wire\n",
                "end\n",
                "fn effected() -> () effects [HTTP::Audit]\n",
                "  ()\n",
                "end\n",
                "fn body() -> Int\n",
                "  1\n",
                "end\n",
                "fn handled() -> Int\n",
                "  handle body() with HTTP::audit()\n",
                "end\n",
            ),
        ),
        SourceFile::new(
            "http.veln",
            concat!(
                "mod transport::http\n",
                "pub type Payload\n",
                "  pub Data(Int)\n",
                "  pub Empty\n",
                "end\n",
                "schema Wire\n",
                "  value: Int\n",
                "end\n",
                "effect Audit\n",
                "  record() -> Int\n",
                "end\n",
                "handler audit() for Audit\n",
                "  record() => 1\n",
                "end\n",
            ),
        ),
    ]);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case")
    );
    for independent in [
        "core.constructor_arity_mismatch",
        "name.unresolved",
        "type.match_non_exhaustive",
        "schema.composition_reference",
        "effect.unknown",
        "handler.unknown",
    ] {
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == independent),
            "missing {independent}: {diagnostics:#?}"
        );
    }
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.message == "unresolved type alias target `HTTP::Missing`"
    }));
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "core.constructor_arity_mismatch"
                && diagnostic.span.as_ref().is_some_and(|span| {
                    span.start.line == 8
                        && span.start.column == 3
                        && span.end.line == 8
                        && span.end.column == 28
                })
                && diagnostic
                    .details
                    .to_json()
                    .contains("\"expected_argument_count\":1")
                && diagnostic
                    .details
                    .to_json()
                    .contains("\"actual_argument_count\":2")
        }),
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.match_non_exhaustive"
            && diagnostic.message == "match is missing case Empty"
    }));
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.message != "unresolved call_target `HTTP::Payload::Data`"
    }));
    assert!(lower_checked_surface_module(&module).core.is_none());
}

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

#[test]
fn combined_module_explicit_alias_casing_diagnostics_use_stable_source_order() {
    fn module(reversed: bool) -> SurfaceModule {
        let first = SourceFile::new(
            "a.veln",
            concat!(
                "mod app\n",
                "use Missing as HTTP::nested\n",
                "use alpha as shared\n",
                "use absent as missing\n",
                "fn unresolved() -> Int\n",
                "  missing::entry()\n",
                "end\n",
            ),
        );
        let second = SourceFile::new(
            "z.veln",
            concat!(
                "mod app\n",
                "use beta as shared\n",
                "use gamma as prelude\n",
            ),
        );
        let sources = if reversed {
            vec![("app", second), ("app", first)]
        } else {
            vec![("app", first), ("app", second)]
        };
        merged_modules_with_identities(sources)
    }

    let diagnostics = [module(false), module(true)].map(|module| analyze_surface_module(&module));
    for observed in &diagnostics {
        assert_eq!(
            observed
                .iter()
                .map(|diagnostic| diagnostic.id.as_str())
                .collect::<Vec<_>>(),
            [
                "name.invalid_case",
                "name.invalid_case",
                "name.duplicate",
                "name.reserved",
                "name.unresolved",
            ],
            "{observed:#?}"
        );
    }
    assert_eq!(
        diagnostics[0]
            .iter()
            .map(veln_diagnostics::diagnostic_to_json)
            .map(|json| json.to_json())
            .collect::<Vec<_>>(),
        diagnostics[1]
            .iter()
            .map(veln_diagnostics::diagnostic_to_json)
            .map(|json| json.to_json())
            .collect::<Vec<_>>()
    );
}
