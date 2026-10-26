use super::*;

#[test]
fn invalid_implicit_import_alias_suppresses_only_quarantine_cascade() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn main() -> Int\n",
            "  HTTP::entry()\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!("mod http\n", "pub fn entry() -> Int\n", "  1\n", "end\n"),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "module.missing_identity"],
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_implicit_import_alias_suppresses_public_effect_quarantine_cascade() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn main() -> () effects [HTTP::Audit]\n",
            "  ()\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!(
            "mod http\n",
            "pub effect Audit\n",
            "  record() -> ()\n",
            "end\n",
        ),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "module.missing_identity"],
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "effect.unknown"),
        "{diagnostics:#?}"
    );
    let environment = TypeEnvironment::from_module(&module);
    let main = environment
        .function("main")
        .expect("valid function signature remains available");
    assert!(main.effects.is_empty());
    assert!(lower_checked_surface_module(&module).core.is_none());
}

#[test]
fn invalid_implicit_import_alias_preserves_missing_private_and_wrong_kind_effects() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn missing() -> () effects [HTTP::Missing]\n",
            "  ()\n",
            "end\n",
            "\n",
            "fn private() -> () effects [HTTP::Audit]\n",
            "  ()\n",
            "end\n",
            "\n",
            "fn wrong_kind() -> () effects [HTTP::entry]\n",
            "  ()\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!(
            "mod http\n",
            "effect Audit\n",
            "  record() -> ()\n",
            "end\n",
            "pub fn entry() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "name.invalid_case",
            "module.missing_identity",
            "effect.unknown",
            "effect.unknown",
            "effect.unknown",
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_implicit_import_alias_suppresses_public_handler_quarantine_cascade() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn body() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn main() -> Int\n",
            "  handle body() with HTTP::audit()\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!(
            "mod http\n",
            "pub effect Audit\n",
            "  record() -> Int\n",
            "end\n",
            "pub handler audit() for Audit\n",
            "  record() => 1\n",
            "end\n",
        ),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "module.missing_identity"],
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "handler.unknown"),
        "{diagnostics:#?}"
    );
    assert!(lower_checked_surface_module(&module).core.is_none());
}

#[test]
fn invalid_implicit_import_alias_preserves_missing_private_and_wrong_kind_handlers() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn body() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn missing() -> Int\n",
            "  handle body() with HTTP::missing()\n",
            "end\n",
            "\n",
            "fn private() -> Int\n",
            "  handle body() with HTTP::private()\n",
            "end\n",
            "\n",
            "fn wrong_kind() -> Int\n",
            "  handle body() with HTTP::Audit()\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!(
            "mod http\n",
            "pub effect Audit\n",
            "  record() -> Int\n",
            "end\n",
            "handler private() for Audit\n",
            "  record() => 1\n",
            "end\n",
        ),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "name.invalid_case",
            "module.missing_identity",
            "handler.unknown",
            "handler.unknown",
            "handler.unknown",
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_implicit_import_alias_preserves_private_call_target() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn main() -> Int\n",
            "  HTTP::entry()\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!("mod http\n", "fn entry() -> Int\n", "  1\n", "end\n"),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "name.invalid_case",
            "module.missing_identity",
            "name.unresolved"
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_implicit_import_alias_suppresses_constructor_quarantine_cascade() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn main() -> HTTP::Payload\n",
            "  HTTP::Payload::Data(1)\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!(
            "mod http\n",
            "pub type Payload\n",
            "  pub Data(Int)\n",
            "end\n",
        ),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    assert_eq!(module.uses[0].alias, "HTTP");
    assert_eq!(module.uses[0].module_name.as_deref(), Some("app"));
    assert_eq!(module.types[0].module_name.as_deref(), Some("HTTP"));
    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment.quarantined_import_constructor_recovery_candidate_count(
            &[
                "HTTP".to_string(),
                "Payload".to_string(),
                "Data".to_string()
            ],
            Some("app"),
            Some(1),
        ),
        1
    );
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "name.unresolved"
                && diagnostic.id != "type.mismatch"
                && diagnostic.id != "core.constructor_arity_mismatch"
        }),
        "{diagnostics:#?}"
    );
    assert!(lower_checked_surface_module(&module).core.is_none());
}

#[test]
fn invalid_implicit_import_alias_does_not_infer_private_signature_type() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn main(value: HTTP::Payload) -> Int\n",
            "  value\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!(
            "mod http\n",
            "pub type Payload\n",
            "  pub Data(Int)\n",
            "end\n",
        ),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.mismatch"),
        "{diagnostics:#?}"
    );
    assert!(
        TypeEnvironment::from_module(&module)
            .uses
            .iter()
            .all(|use_decl| use_decl.alias != "HTTP")
    );
}

#[test]
fn invalid_implicit_import_alias_preserves_private_schema_composition() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "schema Packet\n",
            "  payload: HTTP::Wire\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!("mod http\n", "schema Wire\n", "  payload: Int\n", "end\n"),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case"),
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "name.invalid_case",
            "module.missing_identity",
            "schema.composition_reference"
        ],
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics[2]
            .details
            .to_json()
            .contains("\"reason\":\"private_schema\""),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_implicit_import_alias_preserves_missing_schema_composition() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "schema Packet\n",
            "  payload: HTTP::Missing\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&app_source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "schema.composition_reference"],
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics[1]
            .details
            .to_json()
            .contains("\"reason\":\"missing_schema\""),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_implicit_import_alias_preserves_missing_type_export() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn main(value: HTTP::Payload) -> Int\n",
            "  value\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!(
            "mod http\n",
            "pub type Other\n",
            "  pub Data(Int)\n",
            "end\n",
        ),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "name.invalid_case",
            "module.missing_identity",
            "type.mismatch"
        ],
        "{diagnostics:#?}"
    );
    assert_eq!(
        TypeEnvironment::from_module(&module).quarantined_import_type_recovery_candidate_count(
            "HTTP::Payload",
            Some("app"),
            0,
        ),
        0
    );
}

fn merged_app_and_http_modules(app_source: SourceFile, http_source: SourceFile) -> SurfaceModule {
    let app_module = lower_surface_ast(&parse(&app_source).tree);
    let mut http_module = lower_surface_ast(&parse(&http_source).tree);
    assign_module_name(&mut http_module, "HTTP");
    merge_surface_modules(vec![app_module, http_module])
}

fn assign_module_name(module: &mut SurfaceModule, name: &str) {
    for use_decl in &mut module.uses {
        use_decl.module_name = Some(name.to_string());
    }
    for alias in &mut module.aliases {
        alias.module_name = Some(name.to_string());
    }
    for effect in &mut module.effects {
        effect.module_name = Some(name.to_string());
    }
    for handler in &mut module.handlers {
        handler.module_name = Some(name.to_string());
    }
    for schema in &mut module.schemas {
        schema.module_name = Some(name.to_string());
    }
    for type_decl in &mut module.types {
        type_decl.module_name = Some(name.to_string());
    }
    for function in &mut module.functions {
        function.module_name = Some(name.to_string());
    }
}

fn merge_surface_modules(modules: Vec<SurfaceModule>) -> SurfaceModule {
    let mut merged = SurfaceModule {
        module: None,
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: Vec::new(),
        module_values: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    };
    for module in modules {
        merged.uses.extend(module.uses);
        merged.aliases.extend(module.aliases);
        merged.effects.extend(module.effects);
        merged.handlers.extend(module.handlers);
        merged.schemas.extend(module.schemas);
        merged.types.extend(module.types);
        merged.functions.extend(module.functions);
        merged.invalid_names.extend(module.invalid_names);
    }
    merged
}

#[test]
fn duplicate_invalid_implicit_import_aliases_stay_in_duplicate_analysis() {
    let source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "use HTTP\n",
            "\n",
            "fn main() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "name.invalid_case", "name.duplicate"],
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics[2].message, "duplicate import name `HTTP`",
        "{diagnostics:#?}"
    );
}

#[test]
fn quarantined_import_alias_use_reports_unresolved_when_target_is_missing() {
    let source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use missing::HTTP\n",
            "\n",
            "fn main() -> Int\n",
            "  HTTP::entry()\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "name.unresolved"],
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics[1].message, "unresolved call_target `HTTP::entry`",
        "{diagnostics:#?}"
    );
}

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
