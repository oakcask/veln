use super::{extend, source_module};
use crate::analyze_surface_module;
use crate::types::{HandlerPathResolution, TypeEnvironment, UserEffectPathResolution};

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
