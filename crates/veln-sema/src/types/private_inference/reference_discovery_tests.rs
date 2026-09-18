use super::*;

fn reference_module(body: &str) -> SurfaceModule {
    let text = format!(
        "fn first() -> Int\n  1\nend\n\
         fn second() -> Int\n  2\nend\n\
         fn third() -> Int\n  3\nend\n\
         fn subject()\n{body}\nend\n"
    );
    let source = veln_source::SourceFile::new("references.veln", text);
    let parsed = veln_syntax::parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    veln_ast::lower_surface_ast(&parsed.tree)
}

fn visit_subject(
    module: &SurfaceModule,
    visitor: &mut impl FnMut(FunctionKey) -> ControlFlow<()>,
) -> ControlFlow<()> {
    let functions = function_ast_map(module);
    let subject = functions.get(&(None, "subject".to_string())).unwrap();
    visit_private_function_references(subject, &functions, visitor)
}

#[test]
fn private_reference_discovery_preserves_aliases_and_match_arm_scopes() {
    let module = reference_module(concat!(
        "  let alias = first\n",
        "  match true\n",
        "    first => first\n",
        "    false => [first, alias]\n",
        "  end\n",
        "  first\n",
        "  let alias = second\n",
        "  alias\n",
    ));
    let mut names = Vec::new();
    assert!(
        visit_subject(&module, &mut |key| {
            names.push(key.1);
            ControlFlow::Continue(())
        })
        .is_continue()
    );
    assert_eq!(
        names,
        ["first", "first", "first", "first", "second", "second"]
    );
}

#[test]
fn private_reference_discovery_preserves_child_order_and_skips_type_applied_callee() {
    let module = reference_module(concat!(
        "  [first(second), {value: third}, {[first]: second}, -third, first + second]\n",
        "  if first\n",
        "    second\n",
        "  else if third\n",
        "    first\n",
        "  else\n",
        "    second\n",
        "  end\n",
        "  first<Int>(third)\n",
    ));
    let mut names = Vec::new();
    let _ = visit_subject(&module, &mut |key| {
        names.push(key.1);
        ControlFlow::Continue(())
    });
    assert_eq!(
        names,
        [
            "first", "second", "third", "first", "second", "third", "first", "second", "first",
            "second", "third", "first", "second", "third",
        ]
    );
}

#[test]
fn private_reference_discovery_stops_before_later_children_and_body_lines() {
    let module = reference_module("  [first, {value: second}, third]\n  first\n");
    let mut names = Vec::new();
    assert!(
        visit_subject(&module, &mut |key| {
            let stop = key.1 == "second";
            names.push(key.1);
            if stop {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
        .is_break()
    );
    assert_eq!(names, ["first", "second"]);
}
