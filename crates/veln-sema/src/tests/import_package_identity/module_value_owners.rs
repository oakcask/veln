use super::*;

#[test]
fn module_value_constructor_lowering_keeps_package_distinct_owner_imports() {
    let module = package_distinct_constructor_module();
    let lowered = lower_checked_surface_module(&module);
    assert!(
        lowered.diagnostics.iter().all(|diagnostic| {
            diagnostic.id == "module.value_not_executable"
                || diagnostic.severity != veln_diagnostics::Severity::Error
        }),
        "{:#?}",
        lowered.diagnostics
    );
    let core = lowered
        .core
        .expect("constructors should reach checked core");
    let ir = lowered.ir.expect("constructors should reach typed IR");
    for package in ["one/pkg", "two/pkg"] {
        assert_package_owned_constructor(&core, &ir, package);
    }
}

fn package_distinct_constructor_module() -> SurfaceModule {
    let states = concat!("pub type State\n", "  pub Ready\n", "end\n");
    let shared = concat!("use states\n", "let selected: states::State = Ready\n",);
    let mut one_states = source_module("one/states.veln", states, Some("one/pkg"), "states");
    let mut one_shared = source_module("one/shared.veln", shared, Some("one/pkg"), "shared");
    let mut two_states = source_module("two/states.veln", states, Some("two/pkg"), "states");
    let mut two_shared = source_module("two/shared.veln", shared, Some("two/pkg"), "shared");
    set_declaration_module_name(&mut one_states, "states");
    set_declaration_module_name(&mut one_shared, "shared");
    set_declaration_module_name(&mut two_states, "states");
    set_declaration_module_name(&mut two_shared, "shared");

    let mut module = one_shared;
    for other in [two_states, one_states, two_shared] {
        extend(&mut module, other);
    }
    module
}

fn assert_package_owned_constructor(
    core: &veln_core::CheckedProgram,
    ir: &veln_ir::TypedProgram,
    package: &str,
) {
    let declaration = core
        .module_values
        .iter()
        .find(|value| value.identity.package.as_deref() == Some(package))
        .expect("package-owned core module value");
    assert!(matches!(
        declaration.initializer.kind,
        veln_core::CoreExprKind::AdtVariant { ref payloads, .. } if payloads.is_empty()
    ));
    let declaration = ir
        .module_values
        .iter()
        .find(|value| value.identity.package.as_deref() == Some(package))
        .expect("package-owned IR module value");
    assert!(matches!(
        declaration.initializer.kind,
        veln_ir::IrExprKind::AdtVariant { ref payloads, .. } if payloads.is_empty()
    ));
}

#[test]
fn public_module_value_visibility_uses_complete_package_and_module_owner() {
    let declarations = concat!(
        "use helper\n",
        "type Secret\n",
        "  pub Value\n",
        "end\n",
        "pub type State\n",
        "  Hidden\n",
        "end\n",
        "pub let named: Secret = Value\n",
        "pub let refined: State::Hidden = Hidden\n",
    );

    for reverse in [false, true] {
        assert_module_value_visibility_diagnostics(declarations, reverse);
    }
}

fn assert_module_value_visibility_diagnostics(declarations: &str, reverse: bool) {
    let one = source_module("one/shared.veln", declarations, Some("one/pkg"), "shared");
    let two = source_module("two/shared.veln", declarations, Some("two/pkg"), "shared");
    let mut module = if reverse { two.clone() } else { one.clone() };
    if reverse {
        extend(&mut module, one);
    } else {
        extend(&mut module, two);
    }

    let diagnostics = analyze_surface_module(&module);
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
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "reverse={reverse}: {diagnostics:#?}"
    );
}
