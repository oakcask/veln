use super::*;

fn lower_fixture_expr(
    ty: CoreType,
    build: impl FnOnce(&Expr) -> CoreExprKind,
) -> Result<IrExpr, IrLowerError> {
    let module = lower_source("fn main() -> ()\n  ()\nend\n");
    let anchor = expr_line(&main_function(&module).body[0]);
    lower_expr(&core_expr(anchor, ty, build(anchor)))
}

fn unit_at(anchor: &Expr) -> CoreExpr {
    core_expr(anchor, CoreType::unit(), CoreExprKind::Unit)
}

#[test]
fn lower_record_preserves_fields_types_and_nested_values() {
    let record_type = CoreType::Record(vec![
        ("some".to_string(), CoreType::option(CoreType::bool())),
        ("ratio".to_string(), CoreType::float()),
    ]);
    let record = lower_fixture_expr(record_type.clone(), |anchor| {
        CoreExprKind::Record(vec![
            CoreRecordField {
                node_id: anchor.node_id,
                name: "some".to_string(),
                expr: core_expr(
                    anchor,
                    CoreType::option(CoreType::bool()),
                    CoreExprKind::OptionSome(Box::new(core_expr(
                        anchor,
                        CoreType::bool(),
                        CoreExprKind::BoolLiteral(true),
                    ))),
                ),
                span: anchor.span.clone(),
            },
            CoreRecordField {
                node_id: anchor.node_id,
                name: "ratio".to_string(),
                expr: core_expr(
                    anchor,
                    CoreType::float(),
                    CoreExprKind::FloatLiteral("1.5".to_string()),
                ),
                span: anchor.span.clone(),
            },
        ])
    })
    .expect("record should lower");

    assert_eq!(record.ty, record_type);
    assert!(matches!(
        record.kind,
        IrExprKind::Record(fields)
            if matches!(fields.as_slice(), [some, ratio]
                if some.name == "some"
                    && some.value.ty == CoreType::option(CoreType::bool())
                    && matches!(some.value.kind, IrExprKind::OptionSome(_))
                    && ratio.name == "ratio"
                    && ratio.value.ty == CoreType::float()
                    && ratio.value.kind == IrExprKind::FloatLiteral("1.5".to_string()))
    ));
}

#[test]
fn lower_result_err_preserves_value_and_type() {
    let result_type = CoreType::result(CoreType::Unknown, CoreType::string());
    let result = lower_fixture_expr(result_type.clone(), |anchor| {
        CoreExprKind::ResultErr(Box::new(core_expr(
            anchor,
            CoreType::string(),
            CoreExprKind::StringLiteral("bad".to_string()),
        )))
    })
    .expect("result error should lower");

    assert_eq!(result.ty, result_type);
    assert!(matches!(
        result.kind,
        IrExprKind::ResultErr(value)
            if value.ty == CoreType::string()
                && value.kind == IrExprKind::StringLiteral("bad".to_string())
    ));
}

#[test]
fn lower_list_preserves_items_and_type() {
    let list_type = CoreType::vec(CoreType::int());
    let list = lower_fixture_expr(list_type.clone(), |anchor| {
        CoreExprKind::List(vec![
            core_expr(
                anchor,
                CoreType::int(),
                CoreExprKind::IntLiteral("1".to_string()),
            ),
            core_expr(
                anchor,
                CoreType::int(),
                CoreExprKind::IntLiteral("2".to_string()),
            ),
        ])
    })
    .expect("list should lower");

    assert_eq!(list.ty, list_type);
    assert!(matches!(
        list.kind,
        IrExprKind::List(items)
            if matches!(items.as_slice(), [first, second]
                if first.kind == IrExprKind::IntLiteral("1".to_string())
                    && second.kind == IrExprKind::IntLiteral("2".to_string()))
    ));
}

#[test]
fn lower_try_preserves_call_and_types() {
    let app_error = CoreType::named("AppError", vec![]);
    let tried = lower_fixture_expr(CoreType::int(), |anchor| {
        CoreExprKind::Try(Box::new(core_expr(
            anchor,
            CoreType::result(CoreType::int(), app_error),
            CoreExprKind::Call {
                target: CoreCallTarget::Function("parse".to_string()),
                args: vec![core_expr(
                    anchor,
                    CoreType::string(),
                    CoreExprKind::StringLiteral("1".to_string()),
                )],
            },
        )))
    })
    .expect("try should lower");

    assert_eq!(tried.ty, CoreType::int());
    assert!(matches!(
        tried.kind,
        IrExprKind::Try(value)
            if matches!(value.kind, IrExprKind::Call {
                target: IrCallTarget::Function(ref name),
                ref args,
            } if name == "parse"
                && matches!(args.as_slice(), [arg]
                    if arg.ty == CoreType::string()
                        && arg.kind == IrExprKind::StringLiteral("1".to_string())))
    ));
}

#[test]
fn lower_prefix_preserves_operator_operand_and_type() {
    let negated = lower_fixture_expr(CoreType::int(), |anchor| CoreExprKind::Prefix {
        op: PrefixOp::Negate,
        expr: Box::new(core_expr(
            anchor,
            CoreType::int(),
            CoreExprKind::IntLiteral("1".to_string()),
        )),
    })
    .expect("negation should lower");
    let checked = lower_fixture_expr(CoreType::bool(), |anchor| CoreExprKind::Prefix {
        op: PrefixOp::Not,
        expr: Box::new(core_expr(
            anchor,
            CoreType::bool(),
            CoreExprKind::BoolLiteral(false),
        )),
    })
    .expect("boolean not should lower");

    assert!(matches!(
        negated.kind,
        IrExprKind::Prefix {
            op: PrefixOp::Negate,
            expr,
        } if negated.ty == CoreType::int()
            && expr.kind == IrExprKind::IntLiteral("1".to_string())
    ));
    assert!(matches!(
        checked.kind,
        IrExprKind::Prefix {
            op: PrefixOp::Not,
            expr,
        } if checked.ty == CoreType::bool() && expr.kind == IrExprKind::BoolLiteral(false)
    ));
}

#[test]
fn lower_binary_preserves_operator_operands_and_type() {
    let binary = lower_fixture_expr(CoreType::int(), |anchor| CoreExprKind::Binary {
        op: BinaryOp::Add,
        left: Box::new(core_expr(
            anchor,
            CoreType::int(),
            CoreExprKind::IntLiteral("1".to_string()),
        )),
        right: Box::new(core_expr(
            anchor,
            CoreType::int(),
            CoreExprKind::IntLiteral("2".to_string()),
        )),
    })
    .expect("binary expression should lower");

    assert_eq!(binary.ty, CoreType::int());
    assert!(matches!(
        binary.kind,
        IrExprKind::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } if left.kind == IrExprKind::IntLiteral("1".to_string())
            && right.kind == IrExprKind::IntLiteral("2".to_string())
    ));
}

#[test]
fn lower_list_cons_preserves_head_and_tail() {
    let list_cons = lower_fixture_expr(CoreType::named("List", vec![CoreType::unit()]), |anchor| {
        CoreExprKind::ListCons {
            head: Box::new(unit_at(anchor)),
            tail: Box::new(core_expr(
                anchor,
                CoreType::named("List", vec![CoreType::unit()]),
                CoreExprKind::ListNil,
            )),
        }
    })
    .expect("list cons should lower");
    assert!(matches!(
        list_cons.kind,
        IrExprKind::ListCons { head, tail }
            if matches!(head.kind, IrExprKind::Unit)
                && matches!(tail.kind, IrExprKind::ListNil)
    ));
}

#[test]
fn lower_adt_variant_preserves_name_and_payloads() {
    let variant = lower_fixture_expr(CoreType::named("Choice", vec![]), |anchor| {
        CoreExprKind::AdtVariant {
            name: vec!["Choice".to_string(), "Value".to_string()],
            payloads: vec![unit_at(anchor)],
        }
    })
    .expect("ADT variant should lower");
    assert!(matches!(
        variant.kind,
        IrExprKind::AdtVariant { name, payloads }
            if name == ["Choice", "Value"]
                && matches!(payloads.as_slice(), [IrExpr { kind: IrExprKind::Unit, .. }])
    ));
}

#[test]
fn lower_perform_preserves_effect_operation_and_arguments() {
    let perform = lower_fixture_expr(CoreType::unit(), |anchor| CoreExprKind::Perform {
        effect: "console".to_string(),
        operation: "write".to_string(),
        args: vec![unit_at(anchor)],
    })
    .expect("perform should lower");
    assert!(matches!(
        perform.kind,
        IrExprKind::Perform { effect, operation, args }
            if effect == "console"
                && operation == "write"
                && matches!(args.as_slice(), [IrExpr { kind: IrExprKind::Unit, .. }])
    ));
}

#[test]
fn lower_handle_preserves_providers_context_and_body() {
    let handle = lower_fixture_expr(CoreType::unit(), |anchor| CoreExprKind::Handle {
        effect: "console".to_string(),
        providers: vec![veln_core::CoreHandlerProvider {
            operation: "write".to_string(),
            function: "capture".to_string(),
        }],
        context_args: vec![unit_at(anchor)],
        body: Box::new(unit_at(anchor)),
    })
    .expect("handle should lower");
    assert!(matches!(
        handle.kind,
        IrExprKind::Handle {
            effect,
            providers,
            context_args,
            body,
        } if effect == "console"
            && matches!(providers.as_slice(), [IrHandlerProvider { operation, function }]
                if operation == "write" && function == "capture")
            && matches!(context_args.as_slice(), [IrExpr { kind: IrExprKind::Unit, .. }])
            && matches!(body.kind, IrExprKind::Unit)
    ));
}

#[test]
fn lower_field_access_preserves_base_and_field() {
    let field_access = lower_fixture_expr(CoreType::unit(), |anchor| CoreExprKind::FieldAccess {
        base: Box::new(unit_at(anchor)),
        field: "value".to_string(),
    })
    .expect("field access should lower");
    assert!(matches!(
        field_access.kind,
        IrExprKind::FieldAccess { base, field }
            if field == "value" && matches!(base.kind, IrExprKind::Unit)
    ));
}
