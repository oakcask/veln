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

fn int_at(anchor: &Expr, value: i64) -> CoreExpr {
    if value >= 0 {
        core_expr(
            anchor,
            CoreType::int(),
            CoreExprKind::IntLiteral(value.to_string()),
        )
    } else {
        core_expr(
            anchor,
            CoreType::int(),
            CoreExprKind::Prefix {
                op: PrefixOp::Negate,
                expr: Box::new(core_expr(
                    anchor,
                    CoreType::int(),
                    CoreExprKind::IntLiteral(value.unsigned_abs().to_string()),
                )),
            },
        )
    }
}

fn closed_int_binary(anchor: &Expr, op: BinaryOp, left: i64, right: i64) -> CoreExpr {
    core_expr(
        anchor,
        CoreType::int(),
        CoreExprKind::Binary {
            op,
            left: Box::new(int_at(anchor, left)),
            right: Box::new(int_at(anchor, right)),
        },
    )
}

fn assert_int_binary_folds(op: BinaryOp, left: i64, right: i64, expected: i64) {
    let folded = lower_fixture_expr(CoreType::int(), |anchor| {
        closed_int_binary(anchor, op, left, right).kind
    })
    .expect("integer operator should lower");
    assert_eq!(folded.kind, IrExprKind::IntConstant(expected), "{op:?}");
}

fn assert_int_comparison_folds(op: BinaryOp, left: i64, right: i64, expected: bool) {
    let folded = lower_fixture_expr(CoreType::bool(), |anchor| CoreExprKind::Binary {
        op,
        left: Box::new(int_at(anchor, left)),
        right: Box::new(int_at(anchor, right)),
    })
    .expect("integer comparison should lower");
    assert_eq!(folded.kind, IrExprKind::BoolLiteral(expected), "{op:?}");
}

fn assert_bool_binary_folds(op: BinaryOp, left: bool, right: bool, expected: bool) {
    let folded = lower_fixture_expr(CoreType::bool(), |anchor| CoreExprKind::Binary {
        op,
        left: Box::new(core_expr(
            anchor,
            CoreType::bool(),
            CoreExprKind::BoolLiteral(left),
        )),
        right: Box::new(core_expr(
            anchor,
            CoreType::bool(),
            CoreExprKind::BoolLiteral(right),
        )),
    })
    .expect("boolean operator should lower");
    assert_eq!(folded.kind, IrExprKind::BoolLiteral(expected), "{op:?}");
}

fn balanced_add_tree(anchor: &Expr, depth: usize) -> CoreExpr {
    if depth == 0 {
        return int_at(anchor, 1);
    }
    core_expr(
        anchor,
        CoreType::int(),
        CoreExprKind::Binary {
            op: BinaryOp::Add,
            left: Box::new(balanced_add_tree(anchor, depth - 1)),
            right: Box::new(balanced_add_tree(anchor, depth - 1)),
        },
    )
}

fn deep_add_chain(anchor: &Expr, depth: usize) -> CoreExpr {
    let mut expr = int_at(anchor, 0);
    for _ in 0..depth {
        expr = core_expr(
            anchor,
            CoreType::int(),
            CoreExprKind::Binary {
                op: BinaryOp::Add,
                left: Box::new(expr),
                right: Box::new(int_at(anchor, 1)),
            },
        );
    }
    expr
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
                expr: core_expr(anchor, CoreType::float(), CoreExprKind::FloatLiteral(1.5)),
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
                    && ratio.value.kind == IrExprKind::FloatLiteral(1.5))
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
fn lower_prefix_folds_closed_typed_scalars() {
    let negated = lower_fixture_expr(CoreType::int(), |anchor| CoreExprKind::Prefix {
        op: PrefixOp::Negate,
        expr: Box::new(core_expr(
            anchor,
            CoreType::int(),
            CoreExprKind::IntLiteral("1".to_string()),
        )),
    })
    .expect("negation should lower");
    let inverted = lower_fixture_expr(CoreType::int(), |anchor| CoreExprKind::Prefix {
        op: PrefixOp::BitwiseNot,
        expr: Box::new(core_expr(
            anchor,
            CoreType::int(),
            CoreExprKind::IntLiteral("0".to_string()),
        )),
    })
    .expect("bitwise not should lower");
    let checked = lower_fixture_expr(CoreType::bool(), |anchor| CoreExprKind::Prefix {
        op: PrefixOp::Not,
        expr: Box::new(core_expr(
            anchor,
            CoreType::bool(),
            CoreExprKind::BoolLiteral(false),
        )),
    })
    .expect("boolean not should lower");
    let minimum_negated = lower_fixture_expr(CoreType::int(), |anchor| CoreExprKind::Prefix {
        op: PrefixOp::Negate,
        expr: Box::new(closed_int_binary(anchor, BinaryOp::Add, i64::MAX, 1)),
    })
    .expect("minimum integer negation should lower");

    assert_eq!(negated.ty, CoreType::int());
    assert_eq!(negated.kind, IrExprKind::IntConstant(-1));
    assert_eq!(inverted.ty, CoreType::int());
    assert_eq!(inverted.kind, IrExprKind::IntConstant(-1));
    assert_eq!(checked.ty, CoreType::bool());
    assert_eq!(checked.kind, IrExprKind::BoolLiteral(true));
    assert_eq!(minimum_negated.kind, IrExprKind::IntConstant(i64::MIN));
}

#[test]
fn lower_binary_folds_closed_typed_scalars() {
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
    assert_eq!(binary.kind, IrExprKind::IntConstant(3));
}

#[test]
fn folding_covers_fixed_width_integer_operator_results() {
    for (op, left, right, expected) in [
        (BinaryOp::Add, 7, 9, 16),
        (BinaryOp::Add, i64::MAX, 1, i64::MIN),
        (BinaryOp::Subtract, 7, 9, -2),
        (BinaryOp::Subtract, -9_223_372_036_854_775_807, 2, i64::MAX),
        (BinaryOp::Multiply, i64::MAX, 2, -2),
        (BinaryOp::Divide, -7, 3, -2),
        (BinaryOp::Remainder, -7, 3, -1),
        (BinaryOp::Remainder, 7, -3, 1),
        (BinaryOp::BitwiseAnd, 0b1100, 0b1010, 0b1000),
        (BinaryOp::BitwiseAnd, -1, 1_i64 << 62, 1_i64 << 62),
        (BinaryOp::BitwiseXor, 0b1100, 0b1010, 0b0110),
        (BinaryOp::BitwiseOr, 0b1100, 0b1010, 0b1110),
        (BinaryOp::ShiftLeft, -1, 0, -1),
        (BinaryOp::ShiftLeft, 1, 63, i64::MIN),
        (BinaryOp::ShiftRight, -8, 2, -2),
        (BinaryOp::ShiftRightLogical, -1, 63, 1),
    ] {
        assert_int_binary_folds(op, left, right, expected);
    }
}

#[test]
fn folding_covers_fixed_width_integer_comparisons() {
    for (op, left, right, expected) in [
        (BinaryOp::Equal, 4, 4, true),
        (BinaryOp::NotEqual, 4, 5, true),
        (BinaryOp::Less, -1, 0, true),
        (BinaryOp::LessEqual, 4, 4, true),
        (BinaryOp::Greater, 5, 4, true),
        (BinaryOp::GreaterEqual, 4, 4, true),
    ] {
        assert_int_comparison_folds(op, left, right, expected);
    }
}

#[test]
fn folding_covers_boolean_operator_results() {
    for (op, left, right, expected) in [
        (BinaryOp::And, true, false, false),
        (BinaryOp::Or, true, false, true),
        (BinaryOp::Equal, true, true, true),
        (BinaryOp::NotEqual, true, false, true),
    ] {
        assert_bool_binary_folds(op, left, right, expected);
    }
}

#[test]
fn folding_wraps_minimum_integer_division_and_remainder() {
    fn computed_minimum(anchor: &Expr) -> CoreExpr {
        core_expr(
            anchor,
            CoreType::int(),
            CoreExprKind::Binary {
                op: BinaryOp::Add,
                left: Box::new(core_expr(
                    anchor,
                    CoreType::int(),
                    CoreExprKind::IntLiteral(i64::MAX.to_string()),
                )),
                right: Box::new(core_expr(
                    anchor,
                    CoreType::int(),
                    CoreExprKind::IntLiteral("1".to_string()),
                )),
            },
        )
    }
    fn negative_one(anchor: &Expr) -> CoreExpr {
        core_expr(
            anchor,
            CoreType::int(),
            CoreExprKind::Prefix {
                op: PrefixOp::Negate,
                expr: Box::new(core_expr(
                    anchor,
                    CoreType::int(),
                    CoreExprKind::IntLiteral("1".to_string()),
                )),
            },
        )
    }

    for (op, expected) in [(BinaryOp::Divide, i64::MIN), (BinaryOp::Remainder, 0)] {
        let folded = lower_fixture_expr(CoreType::int(), |anchor| CoreExprKind::Binary {
            op,
            left: Box::new(computed_minimum(anchor)),
            right: Box::new(negative_one(anchor)),
        })
        .expect("minimum integer boundary should lower");
        assert_eq!(folded.kind, IrExprKind::IntConstant(expected));
    }
}

#[test]
fn folding_keeps_failing_parents_with_folded_children_and_original_spans() {
    for op in [
        BinaryOp::Divide,
        BinaryOp::Remainder,
        BinaryOp::ShiftLeft,
        BinaryOp::ShiftRight,
        BinaryOp::ShiftRightLogical,
    ] {
        let residual = lower_fixture_expr(CoreType::int(), |anchor| {
            let invalid = if matches!(op, BinaryOp::Divide | BinaryOp::Remainder) {
                closed_int_binary(anchor, BinaryOp::Subtract, 1, 1)
            } else {
                closed_int_binary(anchor, BinaryOp::Add, 63, 1)
            };
            CoreExprKind::Binary {
                op,
                left: Box::new(int_at(anchor, 7)),
                right: Box::new(invalid),
            }
        })
        .expect("residual failure should lower");
        assert!(matches!(
            &residual.kind,
            IrExprKind::Binary { op: actual, right, .. }
                if *actual == op
                    && matches!(right.kind, IrExprKind::IntConstant(0 | 64))
                    && right.span == residual.span
        ));
    }
}

#[test]
fn folding_keeps_dynamic_parents_with_folded_children() {
    let dynamic = lower_fixture_expr(CoreType::int(), |anchor| CoreExprKind::Binary {
        op: BinaryOp::Add,
        left: Box::new(core_expr(
            anchor,
            CoreType::int(),
            CoreExprKind::Local("x".to_string()),
        )),
        right: Box::new(closed_int_binary(anchor, BinaryOp::Multiply, 2, 3)),
    })
    .expect("dynamic parent should lower");
    assert!(matches!(
        dynamic.kind,
        IrExprKind::Binary { op: BinaryOp::Add, left, right }
            if matches!(left.kind, IrExprKind::Local(ref name) if name == "x")
                && right.kind == IrExprKind::IntConstant(6)
    ));
}

#[test]
fn folding_excludes_float_context_and_cross_type_equality() {
    let float = lower_fixture_expr(CoreType::float(), |anchor| CoreExprKind::Binary {
        op: BinaryOp::Add,
        left: Box::new(core_expr(
            anchor,
            CoreType::float(),
            CoreExprKind::IntLiteral("1".to_string()),
        )),
        right: Box::new(core_expr(
            anchor,
            CoreType::float(),
            CoreExprKind::IntLiteral("2".to_string()),
        )),
    })
    .expect("float operation should lower");
    assert!(matches!(float.kind, IrExprKind::Binary { .. }));

    let mixed = lower_fixture_expr(CoreType::bool(), |anchor| CoreExprKind::Binary {
        op: BinaryOp::Equal,
        left: Box::new(core_expr(
            anchor,
            CoreType::int(),
            CoreExprKind::IntLiteral("1".to_string()),
        )),
        right: Box::new(core_expr(
            anchor,
            CoreType::bool(),
            CoreExprKind::BoolLiteral(true),
        )),
    })
    .expect("mixed equality should lower");
    assert!(matches!(mixed.kind, IrExprKind::Binary { .. }));
}

#[test]
fn folding_visit_work_is_linear() {
    super::constant_folding::take_visits();
    let small = lower_fixture_expr(CoreType::int(), |anchor| balanced_add_tree(anchor, 6).kind)
        .expect("small tree should lower");
    assert_eq!(small.kind, IrExprKind::IntConstant(64));
    let small_visits = super::constant_folding::take_visits();
    assert_eq!(small_visits, 127);
    let large = lower_fixture_expr(CoreType::int(), |anchor| balanced_add_tree(anchor, 7).kind)
        .expect("large tree should lower");
    assert_eq!(large.kind, IrExprKind::IntConstant(128));
    let large_visits = super::constant_folding::take_visits();
    assert_eq!(large_visits, 255);
    assert_eq!(large_visits, small_visits * 2 + 1);
}

#[test]
fn deep_trees_use_the_existing_lowering_walk() {
    let deep = lower_fixture_expr(CoreType::int(), |anchor| deep_add_chain(anchor, 128).kind)
        .expect("deep tree should lower on the existing expression walk");
    assert_eq!(deep.kind, IrExprKind::IntConstant(128));
    assert_eq!(super::constant_folding::take_visits(), 257);
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
        entries: vec![veln_core::CoreHandlerApplication {
            effect: "console".to_string(),
            providers: vec![veln_core::CoreHandlerProvider {
                operation: "write".to_string(),
                function: "capture".to_string(),
            }],
            context_args: vec![unit_at(anchor)],
        }],
        body: Box::new(unit_at(anchor)),
    })
    .expect("handle should lower");
    assert!(matches!(
        handle.kind,
        IrExprKind::Handle {
            entries,
            body,
        } if matches!(entries.as_slice(), [IrHandlerApplication { effect, providers, context_args }]
            if effect == "console"
            && matches!(providers.as_slice(), [IrHandlerProvider { operation, function }]
                if operation == "write" && function == "capture")
            && matches!(context_args.as_slice(), [IrExpr { kind: IrExprKind::Unit, .. }]))
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
