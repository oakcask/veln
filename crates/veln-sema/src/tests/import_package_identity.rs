use super::support::*;
use crate::semantic_model::Type;
use crate::types::TypeEnvironment;

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
    module.module_values.extend(other.module_values);
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
    for value in &mut module.module_values {
        value.module_name = Some(name.to_string());
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

fn same_spelled_module_values_fixture(
    source_order: [usize; 3],
    reverse_imports: bool,
    reverse_declarations: bool,
) -> SurfaceModule {
    let imports = [
        "use shared as first from \"selected/pkg\"\n",
        "use shared as second from \"namesake/pkg\"\n",
    ];
    let imports = if reverse_imports {
        format!("{}{}", imports[1], imports[0])
    } else {
        imports.concat()
    };
    let mut app = source_module(
        "app.veln",
        &format!(
            "{imports}fn first_value() -> first::Marker\n  first::answer\nend\n\
             fn second_value() -> second::Marker\n  second::answer\nend\n"
        ),
        Some("owner/pkg"),
        "app",
    );
    for use_decl in &mut app.uses {
        use_decl.name = "shared".to_string();
    }
    let declarations = concat!(
        "pub type Marker\n  pub Value\nend\n",
        "pub let answer: Marker = Value\n",
        "pub let distraction: Int = 2\n",
    );
    let mut selected = source_module(
        "selected/shared.veln",
        declarations,
        Some("selected/pkg"),
        "shared",
    );
    let mut namesake = source_module(
        "namesake/shared.veln",
        declarations,
        Some("namesake/pkg"),
        "shared",
    );
    set_declaration_module_name(&mut selected, "shared");
    set_declaration_module_name(&mut namesake, "shared");
    if reverse_declarations {
        selected.module_values.reverse();
        namesake.module_values.reverse();
    }
    let sources = [app, selected, namesake];
    let mut merged = sources[source_order[0]].clone();
    for index in source_order.into_iter().skip(1) {
        extend(&mut merged, sources[index].clone());
    }
    merged
}

fn module_value_for_alias<'a>(
    environment: &'a TypeEnvironment,
    alias: &str,
) -> &'a crate::types::ModuleValueSignature {
    match environment.module_value_path_in_owner(
        &[alias.to_string(), "answer".to_string()],
        Some("owner/pkg::app"),
        Some("owner/pkg"),
    ) {
        crate::types::ModuleValueLookup::Available(value) => value,
        _ => panic!("expected {alias} package value"),
    }
}

#[test]
fn same_spelled_module_values_from_distinct_packages_keep_distinct_identity() {
    for source_order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        for reverse_imports in [false, true] {
            for reverse_declarations in [false, true] {
                let app = same_spelled_module_values_fixture(
                    source_order,
                    reverse_imports,
                    reverse_declarations,
                );
                let context = format!(
                    "source_order={source_order:?}, reverse_imports={reverse_imports}, \
                     reverse_declarations={reverse_declarations}"
                );
                let diagnostics = analyze_surface_module(&app);
                assert!(
                    diagnostics.iter().all(|diagnostic| {
                        !matches!(
                            diagnostic.id.as_str(),
                            "name.unresolved" | "name.ambiguous" | "name.duplicate"
                        )
                    }),
                    "{context}: {diagnostics:#?}"
                );

                let environment = TypeEnvironment::from_module(&app);
                for (alias, package) in [("first", "selected/pkg"), ("second", "namesake/pkg")] {
                    let value = module_value_for_alias(&environment, alias);
                    assert_eq!(
                        (
                            value.declaration.package_name.as_deref(),
                            value.declaration.module_name.as_deref(),
                            value.declaration.name.as_deref(),
                        ),
                        (Some(package), Some("shared"), Some("answer")),
                        "alias={alias}, {context}"
                    );
                    assert!(
                        matches!(
                            &value.ty,
                            Type::Named { identity, .. }
                                if identity == &format!("{package}::shared::Marker")
                        ),
                        "alias={alias}, {context}: {:?}",
                        value.ty
                    );
                }

                let lowered = lower_checked_surface_module(&app);
                let core = lowered.core.expect("package identities should reach core");
                let ir = lowered.ir.expect("package identities should reach IR");
                for (function_name, package) in [
                    ("first_value", "selected/pkg"),
                    ("second_value", "namesake/pkg"),
                ] {
                    let function = core
                        .functions
                        .iter()
                        .find(|function| function.name.ends_with(function_name))
                        .expect("core function");
                    assert!(
                        matches!(
                            function.body.last().map(|statement| &statement.kind),
                            Some(veln_core::CoreStmtKind::Return {
                                expr: veln_core::CoreExpr {
                                    kind: veln_core::CoreExprKind::ModuleValue(identity),
                                    ..
                                }
                            }) if identity.package.as_deref() == Some(package)
                                && identity.module.as_deref() == Some("shared")
                                && identity.name == "answer"
                        ),
                        "{function_name}, {context}: {function:#?}"
                    );
                    let function = ir
                        .functions
                        .iter()
                        .find(|function| function.name.ends_with(function_name))
                        .expect("IR function");
                    assert!(
                        matches!(
                            function.body.last().map(|statement| &statement.kind),
                            Some(veln_ir::IrStmtKind::Return {
                                value: veln_ir::IrExpr {
                                    kind: veln_ir::IrExprKind::ModuleValue(identity),
                                    ..
                                }
                            }) if identity.package.as_deref() == Some(package)
                                && identity.module.as_deref() == Some("shared")
                                && identity.name == "answer"
                        ),
                        "{function_name}, {context}: {function:#?}"
                    );
                }
            }
        }
    }
}

#[test]
fn dependency_owned_module_value_annotations_use_imported_sibling_types() {
    let states = concat!(
        "type Secret\n",
        "  pub Value\n",
        "end\n",
        "pub type State\n",
        "  pub Ready\n",
        "  Hidden\n",
        "end\n",
    );
    let shared = concat!(
        "use states\n",
        "pub let ready: states::State::Ready = states::State::Ready\n",
        "pub let nested: {state: states::State::Ready} = {state: states::State::Ready}\n",
        "pub let private_named: {secret: states::Secret} = 0\n",
        "pub let private_variant: Option<states::State::Hidden> = 0\n",
    );

    for reverse in [false, true] {
        let mut one_states = source_module("one/states.veln", states, Some("one/pkg"), "states");
        let mut one_shared = source_module("one/shared.veln", shared, Some("one/pkg"), "shared");
        let mut two_states = source_module("two/states.veln", states, Some("two/pkg"), "states");
        let mut two_shared = source_module("two/shared.veln", shared, Some("two/pkg"), "shared");
        set_declaration_module_name(&mut one_states, "states");
        set_declaration_module_name(&mut one_shared, "shared");
        set_declaration_module_name(&mut two_states, "states");
        set_declaration_module_name(&mut two_shared, "shared");
        assert!(one_shared.types.is_empty());
        assert!(two_shared.types.is_empty());

        let ordered = if reverse {
            [two_shared, one_states, two_states, one_shared]
        } else {
            [one_shared, two_states, one_states, two_shared]
        };
        let mut module = ordered[0].clone();
        for other in ordered.into_iter().skip(1) {
            extend(&mut module, other);
        }

        let diagnostics = analyze_surface_module(&module);
        assert!(
            diagnostics.iter().all(|diagnostic| {
                diagnostic.id != "type.invalid_annotation"
                    && !(diagnostic.id == "name.unresolved"
                        && diagnostic.message.contains("State::Ready"))
            }),
            "reverse={reverse}: {diagnostics:#?}"
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.public_annotation_private")
                .count(),
            2,
            "reverse={reverse}: {diagnostics:#?}"
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
                .count(),
            2,
            "reverse={reverse}: {diagnostics:#?}"
        );
        for package in ["one/pkg::states", "two/pkg::states"] {
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| format!("{diagnostic:?}").contains(package)),
                "missing provenance for {package}: {diagnostics:#?}"
            );
        }
    }
}

#[test]
fn module_value_annotation_does_not_fall_back_to_a_namesake_package() {
    for reverse in [false, true] {
        let one = source_module(
            "one/shared.veln",
            "type Secret\n  Value\nend\n",
            Some("one/pkg"),
            "shared",
        );
        let two = source_module(
            "two/shared.veln",
            "pub let leaked: Secret = 0\n",
            Some("two/pkg"),
            "shared",
        );
        let mut module = if reverse { two.clone() } else { one.clone() };
        if reverse {
            extend(&mut module, one);
        } else {
            extend(&mut module, two);
        }

        let diagnostics = analyze_surface_module(&module);

        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "name.unresolved"
                    && diagnostic.message == "unresolved type `Secret`"
            }),
            "reverse={reverse}: {diagnostics:#?}"
        );
        assert!(
            diagnostics.iter().all(|diagnostic| {
                diagnostic.id != "type.public_annotation_private"
                    && !diagnostic.message.contains("one/pkg::shared::Secret")
                    && diagnostic
                        .related
                        .iter()
                        .all(|related| !format!("{related:?}").contains("one/pkg::shared::Secret"))
            }),
            "reverse={reverse}: {diagnostics:#?}"
        );
    }
}

#[test]
fn exact_import_matching_owner_module_selects_dependency_annotation_identity() {
    for dependency_first in [false, true] {
        let mut owner = source_module(
            "owner/values.veln",
            concat!(
                "use values from \"dep/pkg\"\n",
                "type Secret\n",
                "  Owner\n",
                "end\n",
                "pub let selected: values::Secret = 0\n",
                "pub let rejected: Secret = 0\n",
            ),
            Some("owner/pkg"),
            "values",
        );
        let mut dependency = source_module(
            "dep/values.veln",
            "pub type Secret\n  pub Dependency\nend\n",
            Some("dep/pkg"),
            "values",
        );
        set_declaration_module_name(&mut owner, "values");
        set_declaration_module_name(&mut dependency, "values");
        let mut project = if dependency_first {
            dependency.clone()
        } else {
            owner.clone()
        };
        if dependency_first {
            extend(&mut project, owner);
        } else {
            extend(&mut project, dependency);
        }

        let diagnostics = analyze_surface_module(&project);
        let visibility = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.public_annotation_private")
            .collect::<Vec<_>>();
        assert_eq!(
            visibility.len(),
            1,
            "dependency_first={dependency_first}: {diagnostics:#?}"
        );
        assert!(
            visibility[0].message.contains("owner/pkg::values::Secret"),
            "dependency_first={dependency_first}: {diagnostics:#?}"
        );

        let environment = TypeEnvironment::from_module(&project);
        let selected = match environment.module_value_path_in_owner(
            &["selected".to_string()],
            Some("values"),
            Some("owner/pkg"),
        ) {
            crate::types::ModuleValueLookup::Available(value) => value,
            _ => panic!("expected selected owner value"),
        };
        let rejected = match environment.module_value_path_in_owner(
            &["rejected".to_string()],
            Some("values"),
            Some("owner/pkg"),
        ) {
            crate::types::ModuleValueLookup::Available(value) => value,
            _ => panic!("expected rejected owner value"),
        };
        assert!(matches!(
            &selected.ty,
            Type::Named { identity, .. } if identity == "dep/pkg::values::Secret"
        ));
        assert!(matches!(
            &rejected.ty,
            Type::Named { identity, .. } if identity == "owner/pkg::values::Secret"
        ));
        assert_ne!(selected.ty, rejected.ty);
    }
}

#[test]
fn same_qualified_module_value_path_from_distinct_packages_is_ambiguous() {
    let mut app = source_module(
        "app.veln",
        concat!(
            "use shared from \"one/pkg\"\n",
            "use shared from \"two/pkg\"\n",
            "fn read() -> Int\n  shared::answer\nend\n",
        ),
        Some("owner/pkg"),
        "app",
    );
    app.uses[0].name = "shared".to_string();
    app.uses[1].name = "shared".to_string();
    set_declaration_module_name(&mut app, "app");
    let mut one = source_module(
        "one/shared.veln",
        "pub let answer: Int = 1\n",
        Some("one/pkg"),
        "shared",
    );
    set_declaration_module_name(&mut one, "shared");
    extend(&mut app, one);
    let mut two = source_module(
        "two/shared.veln",
        "pub let answer: Int = 1\n",
        Some("two/pkg"),
        "shared",
    );
    set_declaration_module_name(&mut two, "shared");
    extend(&mut app, two);

    let diagnostics = analyze_surface_module(&app);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.ambiguous")
            .count(),
        1,
        "{diagnostics:#?}"
    );
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

fn effect_alias_fixture(alias: &str, reverse: bool) -> SurfaceModule {
    let other_alias = if alias == "local" { "remote" } else { "local" };
    let mut module = source_module(
        "app.veln",
        &format!(
            "use std::shared as local\nuse shared as remote from \"std\"\n\
             fn retained() -> Int\n  {alias}::read()\nend\n\
             fn handled() -> Int\n  handle {alias}::read() with {alias}::answer()\nend\n\
             fn partial() -> Int\n  handle {alias}::read() with {other_alias}::answer()\nend\n"
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
    module
}

fn assert_effect_alias_signatures(
    module: &SurfaceModule,
    alias: &str,
    label: &str,
    other_label: &str,
    reverse: bool,
) {
    let environment = TypeEnvironment::from_module(module);
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
}

fn assert_lowered_effect_alias_helpers(
    module: &SurfaceModule,
    alias: &str,
    label: &str,
    other_label: &str,
    reverse: bool,
) {
    let lowered = lower_checked_surface_module(module);
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
                && (function.name.ends_with("::helper") || function.name.ends_with("$helper"))
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

#[test]
fn each_effect_alias_excludes_the_other_package_from_inference_and_discharge() {
    for (alias, label, other_label) in [
        ("local", "workspace::std::shared::Ask", "std::shared::Ask"),
        ("remote", "std::shared::Ask", "workspace::std::shared::Ask"),
    ] {
        for reverse in [false, true] {
            let module = effect_alias_fixture(alias, reverse);
            assert_effect_alias_signatures(&module, alias, label, other_label, reverse);
            assert_lowered_effect_alias_helpers(&module, alias, label, other_label, reverse);
        }
    }
}

mod module_value_owners;
mod package_scoped_helpers;
mod quarantine_recovery;
mod schema_type_identity;
