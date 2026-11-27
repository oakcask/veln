use super::*;

#[test]
fn module_initializers_do_not_fold_closed_expressions() {
    let module = lower_source("fn main() -> ()\n  ()\nend\n");
    let surface = main_function(&module);
    let anchor = expr_line(&surface.body[0]);
    let program = program_with_closed_module_initializers(surface, anchor);

    let ir = lower_checked_core(&program).expect("module initializers should lower");
    assert_unfolded_negative(&ir.module_values[0].initializer);
    let IrExprKind::Record(fields) = &ir.module_values[1].initializer.kind else {
        panic!("nested module initializer should remain a record");
    };
    let [field] = fields.as_slice() else {
        panic!("nested module initializer should retain one field");
    };
    assert_unfolded_negative(&field.value);

    let function_body = lower_expr(&negative_int_expression(anchor))
        .expect("function body expression should lower");
    assert_eq!(function_body.kind, IrExprKind::IntConstant(-3));
}

fn program_with_closed_module_initializers(surface: &Function, anchor: &Expr) -> CheckedProgram {
    let record_type = CoreType::Record(vec![("value".to_string(), CoreType::int())]);
    let mut program = complete_program(Vec::new());
    program.module_values.extend([
        module_value_fixture(
            surface,
            "negative",
            "Int",
            CoreType::int(),
            negative_int_expression(anchor),
        ),
        module_value_fixture(
            surface,
            "nested",
            "{value: Int}",
            record_type.clone(),
            core_expr(
                anchor,
                record_type,
                CoreExprKind::Record(vec![CoreRecordField {
                    node_id: anchor.node_id,
                    name: "value".to_string(),
                    expr: negative_int_expression(anchor),
                    span: anchor.span.clone(),
                }]),
            ),
        ),
    ]);
    program
}

fn module_value_fixture(
    surface: &Function,
    name: &str,
    annotation: &str,
    ty: CoreType,
    initializer: CoreExpr,
) -> CoreModuleValueDecl {
    CoreModuleValueDecl {
        node_id: surface.node_id,
        identity: CoreModuleValueIdentity {
            package: None,
            module: Some("main".to_string()),
            name: name.to_string(),
        },
        annotation: annotation.to_string(),
        ty,
        visibility: Visibility::Private,
        initializer,
        span: surface.span.clone(),
    }
}

fn negative_int_expression(anchor: &Expr) -> CoreExpr {
    core_expr(
        anchor,
        CoreType::int(),
        CoreExprKind::Prefix {
            op: PrefixOp::Negate,
            expr: Box::new(core_expr(
                anchor,
                CoreType::int(),
                CoreExprKind::IntLiteral("3".to_string()),
            )),
        },
    )
}

fn assert_unfolded_negative(expression: &IrExpr) {
    assert!(matches!(
        expression.kind,
        IrExprKind::Prefix {
            op: PrefixOp::Negate,
            ref expr,
        } if matches!(expr.kind, IrExprKind::IntLiteral(ref value) if value == "3")
    ));
}
