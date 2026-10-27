use super::support::*;
use crate::semantic_model::Type;
use crate::types::{HandlerPathResolution, TypeEnvironment, UserEffectPathResolution};

fn source_module(path: &str, text: &str, package: Option<&str>, name: &str) -> SurfaceModule {
    let source = SourceFile::new(path, text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let span = source.span(veln_source::TextRange::new(0, 0));
    match package {
        Some(package) => veln_ast::lower_surface_ast_with_package_module_identity(
            &parsed.tree,
            package.to_string(),
            name.to_string(),
            span,
        ),
        None => {
            veln_ast::lower_surface_ast_with_module_identity(&parsed.tree, name.to_string(), span)
        }
    }
}

fn extend(module: &mut SurfaceModule, other: SurfaceModule) {
    module.uses.extend(other.uses);
    module.functions.extend(other.functions);
    module.effects.extend(other.effects);
    module.handlers.extend(other.handlers);
    module.types.extend(other.types);
    module.schemas.extend(other.schemas);
    module.aliases.extend(other.aliases);
}

fn set_declaration_module_name(module: &mut SurfaceModule, name: &str) {
    for use_decl in &mut module.uses {
        use_decl.module_name = Some(name.to_string());
    }
    for function in &mut module.functions {
        function.module_name = Some(name.to_string());
    }
    for effect in &mut module.effects {
        effect.module_name = Some(name.to_string());
    }
    for handler in &mut module.handlers {
        handler.module_name = Some(name.to_string());
    }
    for type_decl in &mut module.types {
        type_decl.module_name = Some(name.to_string());
    }
    for schema in &mut module.schemas {
        schema.module_name = Some(name.to_string());
    }
    for alias in &mut module.aliases {
        alias.module_name = Some(name.to_string());
    }
}

#[test]
fn quarantined_type_and_constructor_recovery_match_only_the_selected_package() {
    for (include_selected, include_namesake, expected) in
        [(true, false, 1), (false, true, 0), (true, true, 1)]
    {
        let mut module = source_module(
            "app.veln",
            "use math as HTTP from \"selected/pkg\"\n",
            None,
            "app",
        );
        module.uses[0].name = "selected/pkg::math".to_string();
        let declaration = concat!("pub type Payload\n", "  pub Data(Int)\n", "end\n",);
        if include_selected {
            extend(
                &mut module,
                source_module(
                    "selected/math.veln",
                    declaration,
                    Some("selected/pkg"),
                    "math",
                ),
            );
        }
        if include_namesake {
            extend(
                &mut module,
                source_module("other/math.veln", declaration, Some("other/pkg"), "math"),
            );
        }
        let environment = TypeEnvironment::from_module(&module);
        let type_path = "HTTP::Payload";
        let constructor_path = [
            "HTTP".to_string(),
            "Payload".to_string(),
            "Data".to_string(),
        ];
        assert_eq!(
            environment.quarantined_import_type_recovery_candidate_count_in_owner(
                type_path,
                Some("app"),
                None,
                0,
            ),
            expected,
            "selected={include_selected}, namesake={include_namesake}",
        );
        assert_eq!(
            environment.quarantined_import_constructor_recovery_candidate_count_in_owner(
                &constructor_path,
                Some("app"),
                None,
                Some(1),
            ),
            expected,
            "selected={include_selected}, namesake={include_namesake}",
        );
        assert_eq!(
            environment
                .quarantined_explicit_import_constructor_in_owner(
                    &constructor_path,
                    Some("app"),
                    None,
                )
                .is_some(),
            include_selected,
        );
        assert!(
            environment
                .uses
                .iter()
                .all(|use_decl| use_decl.alias != "HTTP"),
            "the invalid alias must remain quarantined",
        );
    }
}

#[test]
fn dependency_owned_quarantine_recovery_selects_only_public_targets_in_the_imported_package() {
    let path = ["HTTP".to_string(), "target".to_string()];
    let effect_path = ["HTTP".to_string(), "Ask".to_string()];
    let handler_path = ["HTTP".to_string(), "answer".to_string()];
    for (case, selected_declarations, expected) in [
        (
            "public",
            concat!(
                "pub fn target() -> Int\n  1\nend\n",
                "pub effect Ask\n  value() -> Int\nend\n",
                "pub handler answer() for Ask\n  value() => 1\nend\n",
            ),
            true,
        ),
        ("missing", "pub fn unrelated() -> Int\n  1\nend\n", false),
        (
            "private",
            concat!(
                "fn target() -> Int\n  1\nend\n",
                "effect Ask\n  value() -> Int\nend\n",
                "handler answer() for Ask\n  value() => 1\nend\n",
            ),
            false,
        ),
        (
            "wrong kind",
            concat!(
                "pub type target\n  Value\nend\n",
                "pub type Ask\n  Value\nend\n",
                "pub type answer\n  Value\nend\n",
            ),
            false,
        ),
    ] {
        for reverse in [false, true] {
            let mut module = source_module(
                "owner/app.veln",
                "use math as HTTP from \"selected/pkg\"\n",
                None,
                "app",
            );
            module.uses[0].importing_package = Some("owner/pkg".to_string());
            module.uses[0].name = "selected/pkg::math".to_string();
            let selected = source_module(
                "selected/math.veln",
                selected_declarations,
                Some("selected/pkg"),
                "math",
            );
            let namesake = source_module(
                "namesake/math.veln",
                concat!(
                    "pub fn target() -> Int\n  2\nend\n",
                    "pub effect Ask\n  value() -> Int\nend\n",
                    "pub handler answer() for Ask\n  value() => 2\nend\n",
                ),
                Some("namesake/pkg"),
                "math",
            );
            for declarations in if reverse {
                [namesake, selected]
            } else {
                [selected, namesake]
            } {
                extend(&mut module, declarations);
            }

            let quarantined = crate::name_recovery::quarantined_use_decls(&module);
            assert_eq!(quarantined.len(), 1, "case={case}: {quarantined:#?}");

            let environment = TypeEnvironment::from_module(&module);
            assert_eq!(
                environment.quarantined_import_call_recovery_candidate_count(
                    &path,
                    Some("app"),
                    Some("owner/pkg"),
                    0,
                ),
                usize::from(expected),
                "case={case}, reverse={reverse}, target={:?}, functions={:?}",
                quarantined[0].resolved_target(),
                module
                    .functions
                    .iter()
                    .map(|function| (
                        function.package_name.as_deref(),
                        function.module_name.as_deref(),
                        function.name.as_deref(),
                    ))
                    .collect::<Vec<_>>(),
            );
            assert_eq!(
                environment.quarantined_import_value_recovery_candidate_count(
                    &path,
                    Some("app"),
                    Some("owner/pkg"),
                ),
                usize::from(expected),
                "case={case}, reverse={reverse}",
            );
            assert_eq!(
                matches!(
                    environment.resolve_user_effect_path_in_owner(
                        &effect_path,
                        Some("app"),
                        Some("owner/pkg"),
                    ),
                    UserEffectPathResolution::QuarantinedImportTarget
                ),
                expected,
                "case={case}, reverse={reverse}",
            );
            assert_eq!(
                matches!(
                    environment.handler_path_in_owner(
                        &handler_path,
                        Some("app"),
                        Some("owner/pkg"),
                    ),
                    HandlerPathResolution::QuarantinedImportTarget
                ),
                expected,
                "case={case}, reverse={reverse}",
            );
        }
    }
}

#[test]
fn public_alias_diagnostics_match_only_the_selected_package() {
    let alias_source = concat!(
        "use math as wire from \"selected/pkg\"\n",
        "pub fn renamed = wire::target\n",
        "pub type Renamed = wire::Payload\n",
    );
    let declarations = concat!(
        "pub fn target() -> Int\n  1\nend\n",
        "pub type Payload\n  pub Data\nend\n",
    );
    for (include_selected, expected_unresolved) in [(false, 2), (true, 0)] {
        let mut module = source_module("app.veln", alias_source, None, "app");
        module.uses[0].name = "selected/pkg::math".to_string();
        extend(
            &mut module,
            source_module("other/math.veln", declarations, Some("other/pkg"), "math"),
        );
        if include_selected {
            extend(
                &mut module,
                source_module(
                    "selected/math.veln",
                    declarations,
                    Some("selected/pkg"),
                    "math",
                ),
            );
        }
        let diagnostics = analyze_surface_module(&module);
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.unresolved")
                .count(),
            expected_unresolved,
            "selected={include_selected}: {diagnostics:#?}",
        );
        let environment = TypeEnvironment::from_module(&module);
        assert_eq!(
            environment
                .function_path_in_owner(&["renamed".to_string()], Some("app"), None)
                .is_some(),
            include_selected,
        );
        assert_eq!(
            environment
                .adts
                .descriptor_for_type_path("Renamed", 0, Some("app"), &environment.uses,)
                .is_some(),
            include_selected,
        );
    }
}

#[test]
fn dependency_owned_schema_quarantine_uses_only_the_selected_package() {
    for (case, selected_declarations, recovers) in [
        ("public", "pub schema Packet\n  value: Int\nend\n", true),
        ("missing", "pub schema Other\n  value: Int\nend\n", false),
        ("private", "schema Packet\n  value: Int\nend\n", false),
        ("wrong kind", "pub fn Packet() -> Int\n  1\nend\n", false),
    ] {
        for reverse in [false, true] {
            let mut module = source_module(
                "owner/app.veln",
                concat!(
                    "use math as HTTP from \"selected/pkg\"\n",
                    "pub schema AliasPacket = HTTP::Packet\n",
                    "schema Host\n  nested: HTTP::Packet\nend\n",
                ),
                Some("owner/pkg"),
                "app",
            );
            module.uses[0].name = "selected/pkg::math".to_string();
            let selected = source_module(
                "selected/math.veln",
                selected_declarations,
                Some("selected/pkg"),
                "math",
            );
            let namesake = source_module(
                "namesake/math.veln",
                "pub schema Packet\n  value: Int\nend\n",
                Some("namesake/pkg"),
                "math",
            );
            for declarations in if reverse {
                [namesake, selected]
            } else {
                [selected, namesake]
            } {
                extend(&mut module, declarations);
            }

            let diagnostics = analyze_surface_module(&module);
            let carrier_failures = diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.id != "name.invalid_case" && diagnostic.id != "name.invalid_import"
                })
                .collect::<Vec<_>>();
            assert_eq!(
                carrier_failures.is_empty(),
                recovers,
                "case={case}, reverse={reverse}: {diagnostics:#?}",
            );
            if !recovers {
                assert!(
                    carrier_failures.iter().any(|diagnostic| {
                        matches!(
                            diagnostic.id.as_str(),
                            "name.unresolved" | "name.visibility" | "name.alias_kind"
                        )
                    }),
                    "public alias failure missing for case={case}: {diagnostics:#?}",
                );
                assert!(
                    carrier_failures
                        .iter()
                        .any(|diagnostic| diagnostic.id == "schema.composition_reference"),
                    "composition failure missing for case={case}: {diagnostics:#?}",
                );
            }
        }
    }
}

#[test]
fn each_effect_alias_excludes_the_other_package_from_inference_and_discharge() {
    for (alias, label, other_label) in [
        ("local", "workspace::std::shared::Ask", "std::shared::Ask"),
        ("remote", "std::shared::Ask", "workspace::std::shared::Ask"),
    ] {
        for reverse in [false, true] {
            let mut module = source_module(
                "app.veln",
                &format!(
                    "use std::shared as local\nuse shared as remote from \"std\"\n\
                 fn retained() -> Int\n  {alias}::read()\nend\n\
                 fn handled() -> Int\n  handle {alias}::read() with {alias}::answer()\nend\n\
                 fn partial() -> Int\n  handle {alias}::read() with {}::answer()\nend\n",
                    if alias == "local" { "remote" } else { "local" },
                ),
                None,
                "app",
            );
            module.uses[1].name = "std::shared".to_string();
            let body = "pub effect Ask\n  value() -> Int\nend\n\
                        fn helper() -> Int\n  perform Ask::value()\nend\n\
                        pub fn read() -> Int effects [Ask]\n  helper()\nend\n\
                        pub handler answer() for Ask\n  value() => 41\nend\n";
            let local = source_module("workspace/shared.veln", body, None, "std::shared");
            let remote = source_module("standard/shared.veln", body, Some("std"), "shared");
            for other in if reverse {
                [remote, local]
            } else {
                [local, remote]
            } {
                extend(&mut module, other);
            }
            let environment = TypeEnvironment::from_module(&module);
            for (name, expected) in [
                ("retained", vec![label.to_string()]),
                ("handled", vec![]),
                ("partial", vec![label.to_string()]),
            ] {
                let signature = environment
                    .function_path_in_owner(&[name.to_string()], Some("app"), None)
                    .unwrap();
                assert_eq!(
                    signature.effects, expected,
                    "alias={alias}, function={name}, reverse={reverse}"
                );
                assert!(!signature.effects.iter().any(|effect| effect == other_label));
            }
            let lowered = lower_checked_surface_module(&module);
            assert!(
                lowered.diagnostics.is_empty(),
                "alias={alias}, reverse={reverse}, {:#?}",
                lowered.diagnostics
            );
            let helper_effects = lowered
                .core
                .as_ref()
                .unwrap()
                .functions
                .iter()
                .filter(|function| {
                    function.span.file.as_str().ends_with("shared.veln")
                        && (function.name.ends_with("::helper")
                            || function.name.ends_with("$helper"))
                })
                .map(|function| function.effects.clone())
                .collect::<Vec<_>>();
            assert_eq!(helper_effects.len(), 2);
            assert!(
                helper_effects
                    .iter()
                    .any(|effects| effects == &[label.to_string()])
            );
            assert!(
                helper_effects
                    .iter()
                    .any(|effects| effects == &[other_label.to_string()])
            );
        }
    }
}

#[test]
fn each_schema_alias_selects_only_its_package_helper_and_lowered_target() {
    for (alias, field, selected, excluded) in [
        (
            "local",
            "local_value",
            "workspace::std::shared::Packet",
            "std::shared::Packet",
        ),
        (
            "remote",
            "remote_value",
            "std::shared::Packet",
            "workspace::std::shared::Packet",
        ),
    ] {
        for reverse in [false, true] {
            let mut module = source_module(
                "app.veln",
                &format!(
                    "use std::shared as local\nuse shared as remote from \"std\"\n\
                 pub fn encode_one(value: {{{field}: Int}}) -> Result<ByteChunk, EncodeError>\n\
                   encode {alias}::AliasPacket from value\nend\n",
                ),
                None,
                "app",
            );
            module.uses[1].name = "std::shared".to_string();
            let local = source_module(
                "workspace/shared.veln",
                "pub schema Packet\n  format binary\n  local_value: UInt8\nend\npub schema AliasPacket = Packet\n",
                None,
                "std::shared",
            );
            let remote = source_module(
                "standard/shared.veln",
                "pub schema Packet\n  format binary\n  remote_value: UInt8\nend\npub schema AliasPacket = Packet\n",
                Some("std"),
                "shared",
            );
            for other in if reverse {
                [remote, local]
            } else {
                [local, remote]
            } {
                extend(&mut module, other);
            }
            let environment = TypeEnvironment::from_module(&module);
            let helper = environment
                .schema_encode_signature_in_owner(
                    &[alias.to_string(), "AliasPacket".to_string()],
                    Some("app"),
                    None,
                )
                .unwrap();
            assert_eq!(
                helper.params,
                vec![Type::Record(vec![(field.to_string(), Type::int())])]
            );
            assert_eq!(helper.target_name, format!("schema-encode:{selected}"));
            assert_ne!(helper.target_name, format!("schema-encode:{excluded}"));
            let lowered = lower_checked_surface_module(&module);
            assert!(
                lowered.diagnostics.is_empty(),
                "alias={alias}, reverse={reverse}, {:#?}",
                lowered.diagnostics
            );
            let encoded = lowered
                .core
                .as_ref()
                .unwrap()
                .functions
                .iter()
                .find(|function| function.name == "encode_one")
                .unwrap();
            assert!(format!("{encoded:?}").contains(&format!("SchemaEncode(\"{selected}\")")));
            assert!(!format!("{encoded:?}").contains(&format!("SchemaEncode(\"{excluded}\")")));
        }
    }
}

#[test]
fn each_private_handler_alias_infers_only_its_package_clause_dependencies() {
    for (alias, expected, excluded) in [("local", "stdio", "db"), ("remote", "db", "stdio")] {
        let mut module = source_module(
            "app.veln",
            &format!(
                "use std::shared as local\nuse shared as remote from \"std\"\nfn main() -> Int\n  {alias}::handled()\nend\n",
            ),
            None,
            "app",
        );
        module.uses[1].name = "std::shared".to_string();
        for (package, effect) in [(None, "stdio"), (Some("std"), "db")] {
            let text = format!(
                "pub effect Ask\n  value() -> Int\nend\nfn helper() -> Int effects [{effect}]\n  1\nend\n\
                               pub fn read() -> Int effects [Ask]\n  perform Ask::value()\nend\n\
                               handler answer() for Ask\n  value() => helper()\nend\n\
                               pub fn handled() -> Int effects [{effect}]\n  handle read() with answer()\nend\n"
            );
            extend(
                &mut module,
                source_module(
                    if package.is_some() {
                        "std/shared.veln"
                    } else {
                        "local/shared.veln"
                    },
                    &text,
                    package,
                    if package.is_some() {
                        "shared"
                    } else {
                        "std::shared"
                    },
                ),
            );
        }
        // Access from each declaration owner also proves that same-module private handler lookup keeps its package.
        let environment = TypeEnvironment::from_module(&module);
        let owner_package = if alias == "remote" { Some("std") } else { None };
        let HandlerPathResolution::Found(handler) = environment.handler_path_in_owner(
            &["answer".to_string()],
            Some("std::shared"),
            owner_package,
        ) else {
            panic!("handler should resolve")
        };
        let signature = environment
            .function_path_in_owner(&["main".to_string()], Some("app"), None)
            .unwrap();
        assert_eq!(signature.effects, vec![expected.to_string()]);
        assert_eq!(handler.effects, vec![expected.to_string()]);
        assert!(!handler.effects.iter().any(|effect| effect == excluded));
    }
}

#[test]
fn body_function_type_annotations_keep_the_importing_declaration_package() {
    for reverse in [false, true] {
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
        let environment = TypeEnvironment::from_module(&module);
        let annotation =
            crate::type_syntax::parse_type_or_unknown(Some("fn() -> Int effects [chosen::Ask]"));
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
            let canonical = environment.canonicalize_type_annotation_in_owner(
                annotation.clone(),
                Some("std::owner"),
                package,
            );
            assert_eq!(
                canonical.function_effects(),
                Some(&[selected.to_string()][..])
            );
            let signature = environment
                .function_path_in_owner(&["bind".to_string()], Some("std::owner"), package)
                .unwrap();
            assert_eq!(signature.effects, vec![selected.to_string()]);
            assert!(!signature.effects.iter().any(|label| label == excluded));
            let lowered = lower_checked_surface_module(&module);
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
            let selected = ["one", "two"].map(|suffix| {
                let package = format!("selected/{suffix}");
                let mut target = source_module(
                    &format!("{package}/target.veln"),
                    selected_source,
                    Some(&package),
                    "target",
                );
                set_declaration_module_name(&mut target, "target");
                target
            });
            let namesake = ["one", "two"].map(|suffix| {
                let package = format!("namesake/{suffix}");
                let mut target = source_module(
                    &format!("{package}/target.veln"),
                    namesake_source,
                    Some(&package),
                    "target",
                );
                set_declaration_module_name(&mut target, "target");
                target
            });
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

#[test]
fn format_neutral_schema_adt_lookup_uses_the_import_target_package() {
    for (case, selected_payload, namesake_payload, accepted) in [
        ("selected", Some("Int"), None, true),
        ("namesake", None, Some("Int"), false),
        ("both selected", Some("Int"), Some("fn() -> Int"), true),
        ("both namesake", Some("fn() -> Int"), Some("Int"), false),
    ] {
        for reverse in [false, true] {
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
            let selected = ["one", "two"].map(|suffix| {
                selected_payload.map(|payload| {
                    let package = format!("selected/{suffix}");
                    let mut target = source_module(
                        &format!("{package}/target.veln"),
                        &format!("pub type Payload\n  Value({payload})\nend\n"),
                        Some(&package),
                        "target",
                    );
                    set_declaration_module_name(&mut target, "target");
                    target
                })
            });
            let namesake = ["one", "two"].map(|suffix| {
                namesake_payload.map(|payload| {
                    let package = format!("namesake/{suffix}");
                    let mut target = source_module(
                        &format!("{package}/target.veln"),
                        &format!("pub type Payload\n  Value({payload})\nend\n"),
                        Some(&package),
                        "target",
                    );
                    set_declaration_module_name(&mut target, "target");
                    target
                })
            });
            let declaration_groups = if reverse {
                [namesake, selected]
            } else {
                [selected, namesake]
            };
            for declaration in declaration_groups.into_iter().flatten().flatten() {
                extend(&mut module, declaration);
            }

            let adts = crate::adt::registry::AdtRegistry::from_module(&module);
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
                let decoded =
                    crate::types::schema_types::format_neutral_schema_field_type_for_schema(
                        &module,
                        schema,
                        &adts,
                        "wire::Payload",
                    );
                let encoded =
                    crate::types::schema_types::format_neutral_schema_encode_field_type_for_schema(
                        &module,
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
