use super::support::*;
use crate::semantic_model::Type;
use crate::types::{HandlerPathResolution, TypeEnvironment};

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
    module.schemas.extend(other.schemas);
    module.aliases.extend(other.aliases);
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
