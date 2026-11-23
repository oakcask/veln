use super::*;

#[test]
fn cloned_function_bodies_keep_nested_mutations_and_wire_output_independent() {
    let original = lower_source("fn example() -> List<Int>\n  [1]\nend\n");
    let original_wire = encode_surface_module(&original);
    let mut edited = original.clone();
    let BodyLineKind::Expr { expr } = &mut edited.functions[0].body[0].kind else {
        panic!("list-returning function must contain an expression");
    };
    let ExprKind::List(items) = &mut expr.kind else {
        panic!("list-returning function must retain its nested list expression");
    };
    items[0].kind = ExprKind::IntLiteral("2".into());

    assert_eq!(
        encode_surface_module(&original),
        original_wire,
        "editing a cloned function must not alter the original nested expression"
    );
    let edited_wire = encode_surface_module(&edited);
    assert_ne!(
        edited_wire, original_wire,
        "encode the edited nested expression"
    );
    let decoded = decode_surface_module(&edited_wire).expect("edited function must decode");
    assert_eq!(encode_surface_module(&decoded), edited_wire);
}

#[test]
fn consuming_and_mutating_function_body_iterators_leave_other_clones_unchanged() {
    let original = lower_source("fn example() -> Int\n  1\nend\n");
    let original_wire = encode_surface_module(&original);
    let mut body: FunctionBody = original.functions[0].body.clone().into_iter().collect();
    for line in &mut body {
        let BodyLineKind::Expr { expr } = &mut line.kind else {
            panic!("integer-returning function must contain an expression");
        };
        expr.kind = ExprKind::IntLiteral("2".into());
    }
    let mut edited = original.clone();
    edited.functions[0].body = body;
    assert_eq!(
        encode_surface_module(&original),
        original_wire,
        "body iteration must retain independent ownership of edited expressions"
    );
    assert_ne!(encode_surface_module(&edited), original_wire);
    edited.functions[0].body.clear();
    assert_eq!(
        original.functions[0].body.len(),
        1,
        "clearing an edited body must preserve the original body"
    );
}
