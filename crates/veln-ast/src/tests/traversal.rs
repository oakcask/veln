use super::*;
use std::ops::ControlFlow;

#[test]
fn expression_child_traversal_returns_break_value_and_skips_remaining_children() {
    for (expression, expected) in child_cases() {
        let source = format!("fn example()\n  {expression}\nend\n");
        let module = lower_source(&source);
        let BodyLineKind::Expr { expr } = &module.functions[0].body[0].kind else {
            panic!("expected expression body");
        };
        for stop in 0..=expected.len() {
            let mut children = Vec::new();
            let result = expr.try_for_each_child(&mut |child| {
                children.push(&source[child.span.start.offset..child.span.end.offset]);
                if children.len() == stop + 1 {
                    ControlFlow::Break(child.node_id)
                } else {
                    ControlFlow::Continue(())
                }
            });
            assert_eq!(
                children,
                expected[..expected.len().min(stop + 1)],
                "{expression}"
            );
            if stop < expected.len() {
                let mut child_ids = Vec::new();
                expr.for_each_child(&mut |child| child_ids.push(child.node_id));
                assert_eq!(result, ControlFlow::Break(child_ids[stop]));
            } else {
                assert_eq!(result, ControlFlow::Continue(()));
            }
        }
    }
}

#[test]
fn expression_children_preserve_source_order_without_recursing() {
    for (expression, expected) in child_cases() {
        let source = format!("fn example()\n  {expression}\nend\n");
        let module = lower_source(&source);
        let BodyLineKind::Expr { expr } = &module.functions[0].body[0].kind else {
            panic!("expected expression body");
        };
        let mut children = Vec::new();
        expr.for_each_child(&mut |child| {
            children.push(&source[child.span.start.offset..child.span.end.offset]);
        });
        assert_eq!(children, expected, "{expression}");
    }
}

fn child_cases() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        ("call(first(), second)", vec!["call", "first()", "second"]),
        ("call<Int>(value)", vec!["call<Int>", "value"]),
        ("perform Ask::value(first, second)", vec!["first", "second"]),
        (
            "handle body() with handler(first, second)",
            vec!["body()", "first", "second"],
        ),
        ("decode Packet from input at base", vec!["input", "base"]),
        ("encode Packet from value", vec!["value"]),
        ("value.field", vec!["value"]),
        ("value?", vec!["value"]),
        ("-value", vec!["value"]),
        ("left + right", vec!["left", "right"]),
        ("[first(), second]", vec!["first()", "second"]),
        ("{one: first, two: second}", vec!["first", "second"]),
        ("{1: first, 2: second}", vec!["1", "first", "2", "second"]),
        (
            "match input\n    Some(bound) => first\n    None => second\n  end",
            vec!["input", "first", "second"],
        ),
        (
            "if first\n    second\n  else if third\n    fourth\n  else\n    fifth\n  end",
            vec!["first", "second", "third", "fourth", "fifth"],
        ),
        ("value", vec![]),
        ("42", vec![]),
        ("()", vec![]),
    ]
}
