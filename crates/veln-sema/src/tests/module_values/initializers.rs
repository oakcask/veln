use super::*;

#[test]
fn module_values_retain_checked_core_and_typed_ir_without_diagnostics() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let answer: Int = 42\n",
            "let backup: Int = 7\n",
            "let final_value: Int = 9\n",
            "fn ordinary() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    assert_eq!(module.module_values.len(), 3);
    assert_eq!(module.functions.len(), 1);

    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core");
    let ir = lowered.ir.as_ref().expect("typed IR");
    assert_eq!(core.module_values.len(), 3);
    assert_eq!(ir.module_values.len(), 3);
    assert_eq!(core.module_values[0].identity.name, "answer");
    assert_eq!(ir.module_values[0].identity, core.module_values[0].identity);

    let unrelated = Diagnostic::new(
        "caller.unrelated",
        veln_diagnostics::Severity::Warning,
        DiagnosticKind::Module,
        "caller-owned unrelated warning",
        None,
        veln_diagnostics::JsonValue::Null,
    );
    let lowered_with_warning = lower_analyzed_surface_module(&module, vec![unrelated.clone()]);
    assert_eq!(lowered_with_warning.diagnostics.len(), 1);
    assert_eq!(lowered_with_warning.diagnostics[0].id, unrelated.id);
    assert_eq!(
        lowered_with_warning
            .core
            .as_ref()
            .unwrap()
            .module_values
            .len(),
        3
    );
    assert_eq!(
        lowered_with_warning
            .ir
            .as_ref()
            .unwrap()
            .module_values
            .len(),
        3
    );
}

#[test]
fn module_value_initializers_accept_finite_data_and_forward_references() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Idle\n",
            "  Active(count: Int)\n",
            "end\n",
            "let defaults: { state: State, attempts: Int } = { state: initial, attempts: -3 }\n",
            "let initial: State = Active(3)\n",
            "fn attempts() -> Int\n",
            "  defaults.attempts\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != veln_diagnostics::Severity::Error),
        "{diagnostics:#?}"
    );

    let lowered = lower_checked_surface_module(&module);
    let core = lowered
        .core
        .expect("accepted values should retain checked core");
    let ir = lowered.ir.expect("accepted values should retain typed IR");
    assert_eq!(core.module_values.len(), 2);
    assert_eq!(ir.module_values.len(), 2);
    assert_eq!(
        core.module_values[0].annotation,
        "{ state : State, attempts : Int }"
    );
    assert_eq!(
        ir.module_values[0].annotation,
        core.module_values[0].annotation
    );
    let veln_core::CoreExprKind::Record(fields) = &core.module_values[0].initializer.kind else {
        panic!("expected record initializer");
    };
    assert!(matches!(
        &fields[0].expr.kind,
        veln_core::CoreExprKind::ModuleValue(identity)
            if identity.name == "initial"
    ));
    assert!(matches!(
        &fields[1].expr.kind,
        veln_core::CoreExprKind::Prefix {
            op: veln_ast::PrefixOp::Negate,
            ..
        }
    ));
    assert!(matches!(
        &core.module_values[1].initializer.kind,
        veln_core::CoreExprKind::AdtVariant { name, payloads }
            if name == &["State".to_string(), "Active".to_string()]
                && matches!(payloads.as_slice(), [veln_core::CoreExpr { kind: veln_core::CoreExprKind::IntLiteral(value), .. }] if value == "3")
    ));
    assert_eq!(ir.module_values[0].identity, core.module_values[0].identity);
    assert_eq!(ir.module_values[0].ty, core.module_values[0].ty);
    assert_eq!(
        ir.module_values[0].visibility,
        core.module_values[0].visibility
    );
    assert_eq!(ir.module_values[0].span, core.module_values[0].span);
    assert!(matches!(
        &ir.module_values[0].initializer.kind,
        veln_ir::IrExprKind::Record(fields)
            if matches!(&fields[0].value.kind, veln_ir::IrExprKind::ModuleValue(identity) if identity.name == "initial")
    ));
    assert!(matches!(
        core.functions[0].body.last().map(|statement| &statement.kind),
        Some(veln_core::CoreStmtKind::Return {
            expr: veln_core::CoreExpr {
                kind: veln_core::CoreExprKind::FieldAccess { base, field },
                ..
            }
        }) if field == "attempts"
            && matches!(&base.kind, veln_core::CoreExprKind::ModuleValue(identity) if identity.name == "defaults")
    ));
    assert!(matches!(
        ir.functions[0].body.last().map(|statement| &statement.kind),
        Some(veln_ir::IrStmtKind::Return {
            value: veln_ir::IrExpr {
                kind: veln_ir::IrExprKind::FieldAccess { base, field },
                ..
            }
        }) if field == "attempts"
            && matches!(&base.kind, veln_ir::IrExprKind::ModuleValue(identity) if identity.name == "defaults")
    ));
}

#[test]
fn module_value_initializers_accept_the_complete_finite_data_surface() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Idle\n",
            "  Active(count: Int)\n",
            "end\n",
            "let boolean: Bool = true\n",
            "let text: String = \"ready\"\n",
            "let nothing: () = ()\n",
            "let grouped: Bool = (boolean)\n",
            "let idle: State = Idle\n",
            "let active: State = Active(later)\n",
            "let optional: Option<Int> = Some(later)\n",
            "let outcome: Result<Int, String> = Ok(later)\n",
            "let items: List<Int> = Cons(later, Nil)\n",
            "let nested: { state: State, data: { optional: Option<Int>, items: List<Int> } } = { state: Active(later), data: { optional: Some(later), items: Cons(later, Nil) } }\n",
            "let projected: { kept: Int } = { kept: later, extra: { flag: true }, inferred: Some({ flag: true }) }\n",
            "let later: Int = 3\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == veln_diagnostics::Severity::Error)
        .collect::<Vec<_>>();

    assert!(errors.is_empty(), "{diagnostics:#?}");

    let lowered = lower_checked_surface_module(&module);
    let core = lowered.core.expect("finite initializers should reach core");
    let ir = lowered.ir.expect("finite initializers should reach IR");
    assert_eq!(core.module_values.len(), module.module_values.len());
    assert_eq!(ir.module_values.len(), module.module_values.len());
    assert_core_finite_module_value_initializers(&core);
    assert_ir_finite_module_value_initializers(&ir);
}

fn core_module_value<'a>(
    core: &'a veln_core::CheckedProgram,
    name: &str,
) -> &'a veln_core::CoreModuleValueDecl {
    core.module_values
        .iter()
        .find(|declaration| declaration.identity.name == name)
        .expect("core module value")
}

fn assert_core_finite_module_value_initializers(core: &veln_core::CheckedProgram) {
    assert!(matches!(
        core_module_value(core, "boolean").initializer.kind,
        veln_core::CoreExprKind::BoolLiteral(true)
    ));
    assert!(matches!(
        core_module_value(core, "text").initializer.kind,
        veln_core::CoreExprKind::StringLiteral(_)
    ));
    assert!(matches!(
        core_module_value(core, "nothing").initializer.kind,
        veln_core::CoreExprKind::Unit
    ));
    assert!(
        matches!(core_module_value(core, "grouped").initializer.kind, veln_core::CoreExprKind::ModuleValue(ref identity) if identity.name == "boolean")
    );
    assert!(
        matches!(core_module_value(core, "idle").initializer.kind, veln_core::CoreExprKind::AdtVariant { ref payloads, .. } if payloads.is_empty())
    );
    assert!(
        matches!(core_module_value(core, "active").initializer.kind, veln_core::CoreExprKind::AdtVariant { ref payloads, .. } if payloads.len() == 1)
    );
    assert!(matches!(
        core_module_value(core, "optional").initializer.kind,
        veln_core::CoreExprKind::OptionSome(_)
    ));
    assert!(matches!(
        core_module_value(core, "outcome").initializer.kind,
        veln_core::CoreExprKind::ResultOk(_)
    ));
    assert!(matches!(
        core_module_value(core, "items").initializer.kind,
        veln_core::CoreExprKind::ListCons { .. }
    ));
    assert!(matches!(
        core_module_value(core, "nested").initializer.kind,
        veln_core::CoreExprKind::Record(_)
    ));
    let veln_core::CoreExprKind::Record(fields) =
        &core_module_value(core, "projected").initializer.kind
    else {
        panic!("expected projected record initializer");
    };
    assert_eq!(
        fields[1].expr.ty,
        veln_core::CoreType::Record(vec![("flag".to_string(), veln_core::CoreType::bool(),)])
    );
    assert_eq!(
        fields[2].expr.ty,
        veln_core::CoreType::named(
            "Option",
            vec![veln_core::CoreType::Record(vec![(
                "flag".to_string(),
                veln_core::CoreType::bool(),
            )])],
        )
    );
    assert!(matches!(
        &fields[2].expr.kind,
        veln_core::CoreExprKind::OptionSome(payload)
            if payload.ty == veln_core::CoreType::Record(vec![(
                "flag".to_string(),
                veln_core::CoreType::bool(),
            )])
    ));
}

fn ir_module_value<'a>(
    ir: &'a veln_ir::TypedProgram,
    name: &str,
) -> &'a veln_ir::IrModuleValueDecl {
    ir.module_values
        .iter()
        .find(|declaration| declaration.identity.name == name)
        .expect("IR module value")
}

fn assert_ir_finite_module_value_initializers(ir: &veln_ir::TypedProgram) {
    assert!(matches!(
        ir_module_value(ir, "optional").initializer.kind,
        veln_ir::IrExprKind::OptionSome(_)
    ));
    assert!(matches!(
        ir_module_value(ir, "items").initializer.kind,
        veln_ir::IrExprKind::ListCons { .. }
    ));
    let veln_ir::IrExprKind::Record(fields) = &ir_module_value(ir, "projected").initializer.kind
    else {
        panic!("expected projected record initializer");
    };
    assert_eq!(
        fields[1].value.ty,
        veln_core::CoreType::Record(vec![("flag".to_string(), veln_core::CoreType::bool(),)])
    );
    assert_eq!(
        fields[2].value.ty,
        veln_core::CoreType::named(
            "Option",
            vec![veln_core::CoreType::Record(vec![(
                "flag".to_string(),
                veln_core::CoreType::bool(),
            )])],
        )
    );
}

#[test]
fn contextual_empty_dictionaries_are_not_admitted_as_empty_records() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Container\n",
            "  Wrap(items: Dict<Int, Int>)\n",
            "end\n",
            "pub type ContainerAlias = Container\n",
            "let direct: Dict<Int, Int> = {}\n",
            "let nested: { items: Dict<Int, Int> } = { items: {} }\n",
            "let payload: Container = Wrap({})\n",
            "let mismatched_payload: Int = Wrap({})\n",
            "let aliased: ContainerAlias = Wrap({})\n",
            "let structural: {} = {}\n",
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

    assert_eq!(unsupported.len(), 5, "{diagnostics:#?}");
    assert_eq!(
        unsupported
            .iter()
            .map(|diagnostic| {
                let span = diagnostic.span.as_ref().expect("unsupported span");
                (span.start.line, span.end.offset - span.start.offset)
            })
            .collect::<Vec<_>>(),
        vec![(5, 2), (6, 2), (7, 2), (8, 2), (9, 2)]
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "value.module_initializer_unsupported"
                || diagnostic.span.as_ref() != Some(&module.module_values[5].initializer.span)
        }),
        "{diagnostics:#?}"
    );
}
