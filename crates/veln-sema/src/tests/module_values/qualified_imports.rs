use super::*;

#[test]
fn qualified_same_module_values_resolve_across_declaring_sources() {
    for reverse_sources in [false, true] {
        let consumers = (
            "app::config",
            SourceFile::new(
                "consumers.veln",
                concat!(
                    "let bare: Int = later\n",
                    "let leaf: Int = config::later\n",
                    "let complete: Int = app::config::later\n",
                ),
            ),
        );
        let selected = (
            "app::config",
            SourceFile::new("later.veln", "let later: Int = 2\n"),
        );
        let sources = if reverse_sources {
            vec![selected, consumers]
        } else {
            vec![consumers, selected]
        };
        let module = merged_modules_with_identities(sources);

        let lowered = lower_checked_surface_module(&module);

        assert!(
            lowered
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.severity != veln_diagnostics::Severity::Error),
            "reverse_sources={reverse_sources}: {:#?}",
            lowered.diagnostics
        );
        let core = lowered.core.expect("same-module values should reach core");
        let ir = lowered.ir.expect("same-module values should reach IR");
        let selected_identity = veln_core::CoreModuleValueIdentity {
            package: None,
            module: Some("app::config".to_string()),
            name: "later".to_string(),
        };

        for name in ["bare", "leaf", "complete"] {
            let consumer_identity = veln_core::CoreModuleValueIdentity {
                package: None,
                module: Some("app::config".to_string()),
                name: name.to_string(),
            };
            let declaration = core
                .module_values
                .iter()
                .find(|declaration| declaration.identity == consumer_identity)
                .expect("core consumer declaration");
            assert!(
                matches!(
                    &declaration.initializer.kind,
                    veln_core::CoreExprKind::ModuleValue(identity)
                        if identity == &selected_identity
                ),
                "name={name}, reverse_sources={reverse_sources}: {declaration:#?}"
            );
            let declaration = ir
                .module_values
                .iter()
                .find(|declaration| declaration.identity == consumer_identity)
                .expect("IR consumer declaration");
            assert!(
                matches!(
                    &declaration.initializer.kind,
                    veln_ir::IrExprKind::ModuleValue(identity)
                        if identity == &selected_identity
                ),
                "name={name}, reverse_sources={reverse_sources}: {declaration:#?}"
            );
        }
    }
}

#[test]
fn exact_import_precedes_same_module_leaf_value_qualification() {
    let module = merged_modules_with_identities(vec![
        (
            "app::config",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use config\n",
                    "let answer: Int = config::answer()\n",
                    "fn calls_import() -> Int\n",
                    "  config::answer()\n",
                    "end\n",
                ),
            ),
        ),
        (
            "config",
            SourceFile::new(
                "config.veln",
                concat!("pub fn answer() -> Int\n", "  1\n", "end\n"),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "value.module_initializer_unsupported")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            !matches!(
                diagnostic.id.as_str(),
                "type.not_callable" | "value.module_initializer_cycle" | "name.unresolved"
            )
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn same_module_leaf_qualification_rejects_unwritten_prefixes() {
    let module = merged_modules_with_identities(vec![
        (
            "app::config",
            SourceFile::new("first.veln", "let first: Int = bogus::config::later\n"),
        ),
        (
            "app::config",
            SourceFile::new("later.veln", "let later: Int = 2\n"),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved value `bogus::config::later`"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn module_value_initializers_report_smallest_unsupported_expressions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn compute() -> Int\n",
            "  1\n",
            "end\n",
            "let computed: Int = compute()\n",
            "let nested: { value: Int } = { value: 1 + 2 }\n",
            "let floating: Float = 1.0\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let unsupported = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "value.module_initializer_unsupported")
        .collect::<Vec<_>>();

    assert_eq!(unsupported.len(), 3, "{diagnostics:#?}");
    assert_diagnostic_span(unsupported[0], 4, 21, 4, 30);
    assert_diagnostic_span(unsupported[1], 5, 39, 5, 44);
    assert_diagnostic_span(unsupported[2], 6, 23, 6, 26);
    for (index, (diagnostic, name)) in unsupported
        .iter()
        .zip(["computed", "nested", "floating"])
        .enumerate()
    {
        let detail = |field: &str| {
            diagnostic.details.as_object().and_then(|entries| {
                entries
                    .iter()
                    .find(|(entry, _)| entry == field)
                    .and_then(|(_, value)| value.as_text())
            })
        };
        assert_eq!(detail("phase"), Some("type_check"));
        assert_eq!(detail("name"), Some(name));
        assert_eq!(
            diagnostic.related,
            vec![
                veln_diagnostics::JsonValue::object([
                    (
                        "kind",
                        veln_diagnostics::JsonValue::string("module_value_declaration"),
                    ),
                    (
                        "message",
                        veln_diagnostics::JsonValue::string(format!(
                            "Module value `{name}` is declared here."
                        )),
                    ),
                    (
                        "span",
                        crate::diagnostics::span_json(&module.module_values[index].span),
                    ),
                ]),
                veln_diagnostics::JsonValue::object([
                    (
                        "kind",
                        veln_diagnostics::JsonValue::string("permitted_initializers"),
                    ),
                    (
                        "message",
                        veln_diagnostics::JsonValue::string(
                            "Use literals, records, ADT constructors, or eligible same-module or imported module values.",
                        ),
                    ),
                ]),
            ]
        );
    }
}

#[test]
fn imported_module_values_follow_bare_qualified_and_alias_exposure() {
    let module = merged_modules_with_identities(vec![
        (
            "values",
            SourceFile::new("values.veln", "pub let answer: Int = 42\n"),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use values\n",
                    "let bare: Int = answer\n",
                    "let qualified: Int = values::answer\n",
                    "fn read() -> Int\n",
                    "  values::answer\n",
                    "end\n",
                ),
            ),
        ),
        (
            "aliased",
            SourceFile::new(
                "aliased.veln",
                concat!(
                    "use values as config\n",
                    "let qualified: Int = config::answer\n",
                    "fn bare_is_hidden() -> Int\n",
                    "  answer\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved value `answer`"
        }),
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.unresolved")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "value.module_initializer_unsupported"
                && diagnostic.id != "type.mismatch"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn bare_and_qualified_paths_select_the_same_module_value_declaration() {
    let module = merged_modules_with_identities(vec![
        (
            "values",
            SourceFile::new("values.veln", "pub let answer: Int = 42\n"),
        ),
        ("app", SourceFile::new("app.veln", "use values\n")),
    ]);
    let environment = crate::types::TypeEnvironment::from_module(&module);
    let bare =
        match environment.module_value_path_in_owner(&["answer".to_string()], Some("app"), None) {
            crate::types::ModuleValueLookup::Available(value) => value,
            _ => panic!("expected bare declaration"),
        };
    let qualified = match environment.module_value_path_in_owner(
        &["values".to_string(), "answer".to_string()],
        Some("app"),
        None,
    ) {
        crate::types::ModuleValueLookup::Available(value) => value,
        _ => panic!("expected qualified declaration"),
    };

    assert!(std::ptr::eq(bare, qualified));
}

#[test]
fn imported_module_value_spellings_retain_the_selected_identity_in_core_and_ir() {
    let module = merged_modules_with_identities(vec![
        (
            "values",
            SourceFile::new("values.veln", "pub let answer: Int = 42\n"),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use values\n",
                    "let bare: Int = answer\n",
                    "let qualified: Int = values::answer\n",
                ),
            ),
        ),
        (
            "aliased",
            SourceFile::new(
                "aliased.veln",
                concat!(
                    "use values as config\n",
                    "let aliased: Int = config::answer\n",
                ),
            ),
        ),
    ]);

    let lowered = lower_checked_surface_module(&module);
    assert!(
        lowered
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != veln_diagnostics::Severity::Error),
        "{:#?}",
        lowered.diagnostics
    );
    let core = lowered.core.expect("imported values should reach core");
    let ir = lowered.ir.expect("imported values should reach IR");
    let expected = veln_core::CoreModuleValueIdentity {
        package: None,
        module: Some("values".to_string()),
        name: "answer".to_string(),
    };
    for name in ["bare", "qualified", "aliased"] {
        let declaration = core
            .module_values
            .iter()
            .find(|declaration| declaration.identity.name == name)
            .expect("consumer declaration");
        assert!(matches!(
            &declaration.initializer.kind,
            veln_core::CoreExprKind::ModuleValue(identity) if identity == &expected
        ));
        let declaration = ir
            .module_values
            .iter()
            .find(|declaration| declaration.identity.name == name)
            .expect("IR consumer declaration");
        assert!(matches!(
            &declaration.initializer.kind,
            veln_ir::IrExprKind::ModuleValue(identity) if identity == &expected
        ));
    }
}

#[test]
fn qualified_module_value_exact_path_no_fallback_is_stable() {
    let analyze = |reverse_sources: bool, reverse_imports: bool| {
        let alpha_exact = (
            "alpha::exact",
            SourceFile::new("alpha/exact.veln", "pub let other: Int = 1\n"),
        );
        let beta_exact = (
            "beta::exact",
            SourceFile::new("beta/exact.veln", "pub let answer: Int = 1\n"),
        );
        let imports = if reverse_imports {
            "use beta::exact\nuse alpha::exact\n"
        } else {
            "use alpha::exact\nuse beta::exact\n"
        };
        let app = (
            "app",
            SourceFile::new(
                "app.veln",
                format!("{imports}fn exact() -> Int\n  alpha::exact::answer\nend\n"),
            ),
        );
        let sources = if reverse_sources {
            vec![beta_exact, alpha_exact, app]
        } else {
            vec![alpha_exact, beta_exact, app]
        };
        analyze_surface_module(&merged_modules_with_identities(sources))
            .into_iter()
            .filter(|diagnostic| {
                matches!(diagnostic.id.as_str(), "name.ambiguous" | "name.unresolved")
            })
            .map(|diagnostic| (diagnostic.id, diagnostic.message))
            .collect::<Vec<_>>()
    };

    let expected = analyze(false, false);
    assert_eq!(expected.len(), 1, "{expected:#?}");
    assert!(expected.iter().any(|(id, message)| {
        id == "name.unresolved" && message.contains("alpha::exact::answer")
    }));
    assert_eq!(analyze(true, false), expected);
    assert_eq!(analyze(false, true), expected);
    assert_eq!(analyze(true, true), expected);
}
