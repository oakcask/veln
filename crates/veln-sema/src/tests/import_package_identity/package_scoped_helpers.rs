use super::*;
use crate::types::HandlerPathResolution;

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
            assert_schema_alias(alias, field, selected, excluded, reverse);
        }
    }
}

fn assert_schema_alias(alias: &str, field: &str, selected: &str, excluded: &str, reverse: bool) {
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
    assert_schema_helper(&module, alias, field, selected, excluded);
    assert_lowered_schema_target(&module, alias, selected, excluded, reverse);
}

fn assert_schema_helper(
    module: &SurfaceModule,
    alias: &str,
    field: &str,
    selected: &str,
    excluded: &str,
) {
    let environment = TypeEnvironment::from_module(module);
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
}

fn assert_lowered_schema_target(
    module: &SurfaceModule,
    alias: &str,
    selected: &str,
    excluded: &str,
    reverse: bool,
) {
    let lowered = lower_checked_surface_module(module);
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

#[test]
fn each_private_handler_alias_infers_only_its_package_clause_dependencies() {
    for (alias, expected, excluded) in [("local", "stdio", "db"), ("remote", "db", "stdio")] {
        let module = handler_alias_fixture(alias);
        assert_handler_alias(&module, alias, expected, excluded);
    }
}

fn handler_alias_fixture(alias: &str) -> SurfaceModule {
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
        let path = if package.is_some() {
            "std/shared.veln"
        } else {
            "local/shared.veln"
        };
        let name = if package.is_some() {
            "shared"
        } else {
            "std::shared"
        };
        extend(&mut module, source_module(path, &text, package, name));
    }
    module
}

fn assert_handler_alias(module: &SurfaceModule, alias: &str, expected: &str, excluded: &str) {
    let environment = TypeEnvironment::from_module(module);
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
