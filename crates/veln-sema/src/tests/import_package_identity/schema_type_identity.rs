use super::*;

fn body_annotation_owner_fixture(reverse: bool) -> SurfaceModule {
    let text = "use std::signals as chosen\n\
                pub fn bind(callback: fn() -> Int effects [chosen::Ask]) -> Int effects [chosen::Ask]\n\
                  let typed: fn() -> Int effects [chosen::Ask] = callback\n\
                  typed()\nend\n";
    let local = source_module("workspace/owner.veln", text, None, "std::owner");
    let remote = source_module("standard/owner.veln", text, Some("std"), "owner");
    let effect = "pub effect Ask\n  value() -> Int\nend\n";
    let mut module = source_module("app.veln", "", None, "app");
    for owner in if reverse {
        [remote, local]
    } else {
        [local, remote]
    } {
        extend(&mut module, owner);
    }
    extend(
        &mut module,
        source_module("workspace/signals.veln", effect, None, "std::signals"),
    );
    extend(
        &mut module,
        source_module("standard/signals.veln", effect, Some("std"), "signals"),
    );
    module
}

fn assert_body_annotation_owner(
    module: &SurfaceModule,
    reverse: bool,
    package: Option<&str>,
    selected: &str,
    excluded: &str,
    source: &str,
) {
    let environment = TypeEnvironment::from_module(module);
    let annotation =
        crate::type_syntax::parse_type_or_unknown(Some("fn() -> Int effects [chosen::Ask]"));
    let canonical =
        environment.canonicalize_type_annotation_in_owner(annotation, Some("std::owner"), package);
    assert_eq!(
        canonical.function_effects(),
        Some(&[selected.to_string()][..])
    );
    let signature = environment
        .function_path_in_owner(&["bind".to_string()], Some("std::owner"), package)
        .unwrap();
    assert_eq!(signature.effects, vec![selected.to_string()]);
    assert!(!signature.effects.iter().any(|label| label == excluded));

    let lowered = lower_checked_surface_module(module);
    assert!(
        lowered.diagnostics.is_empty(),
        "reverse={reverse}, {:#?}",
        lowered.diagnostics
    );
    let function = lowered
        .core
        .as_ref()
        .unwrap()
        .functions
        .iter()
        .find(|function| function.span.file.as_str() == source)
        .unwrap();
    let CoreStmtKind::Let {
        ty: CoreType::Function { effects, .. },
        ..
    } = &function.body[0].kind
    else {
        panic!("expected function-typed binding")
    };
    assert_eq!(effects, &[selected.to_string()]);
    assert!(!effects.iter().any(|label| label == excluded));
}

#[test]
fn body_function_type_annotations_keep_the_importing_declaration_package() {
    for reverse in [false, true] {
        let module = body_annotation_owner_fixture(reverse);
        for (package, selected, excluded, source) in [
            (
                None,
                "workspace::std::signals::Ask",
                "std::signals::Ask",
                "workspace/owner.veln",
            ),
            (
                Some("std"),
                "std::signals::Ask",
                "workspace::std::signals::Ask",
                "standard/owner.veln",
            ),
        ] {
            assert_body_annotation_owner(&module, reverse, package, selected, excluded, source);
        }
    }
}

#[test]
fn schema_type_references_use_each_declaration_owner_package() {
    let owner_source = concat!(
        "use target as wire from \"TARGET_PACKAGE\"\n",
        "effect Ask\n",
        "  value() -> ()\n",
        "end\n",
        "fn inspect(value: wire::Packet) -> wire::Packet\n",
        "  let local: wire::Packet = value\n",
        "  value\n",
        "end\n",
        "handler answer() for Ask\n",
        "  value() => begin\n",
        "    let local: wire::Packet = ()\n",
        "    ()\n",
        "  end\n",
        "end\n",
        "type Boxed\n",
        "  Value(wire::Packet)\n",
        "end\n",
    );
    let schema_source = "pub schema Packet\n  value: Int\nend\n";

    for (case, include_selected, include_namesake, expected) in [
        ("selected", true, false, 10),
        ("namesake", false, true, 0),
        ("both", true, true, 10),
    ] {
        for reverse in [false, true] {
            let mut module = source_module("root.veln", "", None, "root");
            for (owner, target_package) in
                [("owner/one", "selected/one"), ("owner/two", "selected/two")]
            {
                let mut owner_module = source_module(
                    &format!("{owner}/app.veln"),
                    &owner_source.replace("TARGET_PACKAGE", target_package),
                    Some(owner),
                    "app",
                );
                set_declaration_module_name(&mut owner_module, "app");
                extend(&mut module, owner_module);
            }
            let selected = ["selected/one", "selected/two"].map(|package| {
                let mut target = source_module(
                    &format!("{package}/target.veln"),
                    schema_source,
                    Some(package),
                    "target",
                );
                set_declaration_module_name(&mut target, "target");
                target
            });
            let namesake = ["namesake/one", "namesake/two"].map(|package| {
                let mut target = source_module(
                    &format!("{package}/target.veln"),
                    schema_source,
                    Some(package),
                    "target",
                );
                set_declaration_module_name(&mut target, "target");
                target
            });
            let groups = if reverse {
                [(include_namesake, namesake), (include_selected, selected)]
            } else {
                [(include_selected, selected), (include_namesake, namesake)]
            };
            for (include, declarations) in groups {
                if include {
                    for declaration in declarations {
                        extend(&mut module, declaration);
                    }
                }
            }

            let diagnostics = analyze_surface_module(&module);
            let schema_references = diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.schema_reference")
                .collect::<Vec<_>>();
            assert_eq!(
                schema_references.len(),
                expected,
                "case={case}, reverse={reverse}: {diagnostics:#?}",
            );
            assert_eq!(
                schema_references
                    .iter()
                    .filter(|diagnostic| {
                        diagnostic
                            .details
                            .to_json()
                            .contains("\"use_kind\":\"parameter_type\"")
                    })
                    .count(),
                expected / 5,
                "case={case}, reverse={reverse}",
            );
        }
    }
}

fn schema_type_reference_precedence_fixture(
    selected_source: &str,
    namesake_source: &str,
    reverse: bool,
) -> SurfaceModule {
    let mut module = source_module("root.veln", "", None, "root");
    for suffix in ["one", "two"] {
        let owner_package = format!("owner/{suffix}");
        let target_package = format!("selected/{suffix}");
        let mut owner = source_module(
            &format!("{owner_package}/app.veln"),
            &format!(
                "use target as wire from \"{target_package}\"\n\
                 fn inspect(value: wire::Packet) -> ()\n  ()\nend\n"
            ),
            Some(&owner_package),
            "app",
        );
        set_declaration_module_name(&mut owner, "app");
        extend(&mut module, owner);
    }
    let selected = package_targets("selected", selected_source);
    let namesake = package_targets("namesake", namesake_source);
    for target in if reverse {
        [namesake, selected]
    } else {
        [selected, namesake]
    }
    .into_iter()
    .flatten()
    {
        extend(&mut module, target);
    }
    module
}

fn package_targets(package_prefix: &str, source: &str) -> [SurfaceModule; 2] {
    ["one", "two"].map(|suffix| {
        let package = format!("{package_prefix}/{suffix}");
        let mut target = source_module(
            &format!("{package}/target.veln"),
            source,
            Some(&package),
            "target",
        );
        set_declaration_module_name(&mut target, "target");
        target
    })
}

#[test]
fn schema_type_reference_precedence_uses_the_selected_package() {
    let schema = "pub schema Packet\n  value: Int\nend\n";
    let ordinary_type = "pub type Packet\n  Value(Int)\nend\n";
    let type_alias = concat!(
        "pub type Payload\n  Value(Int)\nend\n",
        "pub type Packet = Payload\n",
    );
    for (case, selected_source, namesake_source, expected) in [
        ("selected type", ordinary_type, schema, 0),
        ("selected alias", type_alias, schema, 0),
        ("selected schema over type", schema, ordinary_type, 2),
        ("selected schema over alias", schema, type_alias, 2),
    ] {
        for reverse in [false, true] {
            let module =
                schema_type_reference_precedence_fixture(selected_source, namesake_source, reverse);
            let diagnostics = analyze_surface_module(&module);
            assert_eq!(
                diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.id == "type.schema_reference")
                    .count(),
                expected,
                "case={case}, reverse={reverse}: {diagnostics:#?}",
            );
        }
    }
}

fn format_neutral_schema_lookup_fixture(
    selected_payload: Option<&str>,
    namesake_payload: Option<&str>,
    reverse: bool,
) -> SurfaceModule {
    let mut module = source_module("root.veln", "", None, "root");
    for suffix in ["one", "two"] {
        let owner_package = format!("owner/{suffix}");
        let target_package = format!("selected/{suffix}");
        let mut owner = source_module(
            &format!("{owner_package}/app.veln"),
            &format!(
                "use target as wire from \"{target_package}\"\n\
                 schema Envelope\n  value: wire::Payload\nend\n"
            ),
            Some(&owner_package),
            "app",
        );
        set_declaration_module_name(&mut owner, "app");
        extend(&mut module, owner);
    }
    let groups = if reverse {
        [
            ("namesake", namesake_payload),
            ("selected", selected_payload),
        ]
    } else {
        [
            ("selected", selected_payload),
            ("namesake", namesake_payload),
        ]
    };
    for (package_prefix, payload) in groups {
        extend_format_neutral_schema_targets(&mut module, package_prefix, payload);
    }
    module
}

fn extend_format_neutral_schema_targets(
    module: &mut SurfaceModule,
    package_prefix: &str,
    payload: Option<&str>,
) {
    let Some(payload) = payload else {
        return;
    };
    for suffix in ["one", "two"] {
        let package = format!("{package_prefix}/{suffix}");
        let mut target = source_module(
            &format!("{package}/target.veln"),
            &format!("pub type Payload\n  Value({payload})\nend\n"),
            Some(&package),
            "target",
        );
        set_declaration_module_name(&mut target, "target");
        extend(module, target);
    }
}

fn assert_format_neutral_schema_lookup(
    module: &SurfaceModule,
    case: &str,
    reverse: bool,
    accepted: bool,
) {
    let adts = crate::adt::registry::AdtRegistry::from_module(module);
    for suffix in ["one", "two"] {
        let owner_package = format!("owner/{suffix}");
        let schema = module
            .schemas
            .iter()
            .find(|schema| {
                schema.name.as_deref() == Some("Envelope")
                    && schema.package_name.as_deref() == Some(&owner_package)
            })
            .unwrap();
        let decoded = crate::types::schema_types::format_neutral_schema_field_type_for_schema(
            module,
            schema,
            &adts,
            "wire::Payload",
        );
        let encoded =
            crate::types::schema_types::format_neutral_schema_encode_field_type_for_schema(
                module,
                schema,
                &adts,
                "wire::Payload",
            );
        assert_eq!(
            decoded.is_some(),
            accepted,
            "decode case={case}, owner={suffix}, reverse={reverse}: {decoded:#?}",
        );
        assert_eq!(
            encoded.is_some(),
            accepted,
            "encode case={case}, owner={suffix}, reverse={reverse}: {encoded:#?}",
        );
        if let Some(Type::Named { identity, .. }) = decoded {
            assert_eq!(identity, format!("selected/{suffix}::target::Payload"));
        }
    }
}

#[test]
fn format_neutral_schema_adt_lookup_uses_the_import_target_package() {
    for (case, selected_payload, namesake_payload, accepted) in [
        ("selected", Some("Int"), None, true),
        ("namesake", None, Some("Int"), false),
        ("both selected", Some("Int"), Some("fn() -> Int"), true),
        ("both namesake", Some("fn() -> Int"), Some("Int"), false),
    ] {
        for reverse in [false, true] {
            let module =
                format_neutral_schema_lookup_fixture(selected_payload, namesake_payload, reverse);
            assert_format_neutral_schema_lookup(&module, case, reverse, accepted);
        }
    }
}

#[test]
fn format_neutral_schema_adt_cache_separates_same_named_packages() {
    for supported_first in [false, true] {
        for reverse_sources in [false, true] {
            let (left_payload, right_payload) = if supported_first {
                ("Int", "fn() -> Int")
            } else {
                ("fn() -> Int", "Int")
            };
            let mut module = source_module(
                "owner/app.veln",
                concat!(
                    "use target as left from \"left/pkg\"\n",
                    "use target as right from \"right/pkg\"\n",
                    "schema Envelope\n",
                    "  value: {first: left::Payload, second: right::Payload}\n",
                    "end\n",
                ),
                Some("owner/pkg"),
                "app",
            );
            set_declaration_module_name(&mut module, "app");
            let mut left = source_module(
                "left/target.veln",
                &format!("pub type Payload\n  Value({left_payload})\nend\n"),
                Some("left/pkg"),
                "target",
            );
            set_declaration_module_name(&mut left, "target");
            let mut right = source_module(
                "right/target.veln",
                &format!("pub type Payload\n  Value({right_payload})\nend\n"),
                Some("right/pkg"),
                "target",
            );
            set_declaration_module_name(&mut right, "target");
            for target in if reverse_sources {
                [right, left]
            } else {
                [left, right]
            } {
                extend(&mut module, target);
            }

            let adts = crate::adt::registry::AdtRegistry::from_module(&module);
            let schema = &module.schemas[0];
            assert!(
                crate::types::schema_types::format_neutral_schema_field_type_for_schema(
                    &module,
                    schema,
                    &adts,
                    &schema.fields[0].ty,
                )
                .is_none(),
                "decode supported_first={supported_first}, reverse_sources={reverse_sources}",
            );
            assert!(
                crate::types::schema_types::format_neutral_schema_encode_field_type_for_schema(
                    &module,
                    schema,
                    &adts,
                    &schema.fields[0].ty,
                )
                .is_none(),
                "encode supported_first={supported_first}, reverse_sources={reverse_sources}",
            );
        }
    }
}
