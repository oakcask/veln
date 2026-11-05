use super::super::*;

#[test]
fn complete_dependency_discovery_finds_hidden_cycles_and_respects_local_shadowing() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn keep(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "let operator_left: Int = operator_right + 1\n",
            "let operator_right: Int = operator_left\n",
            "let call_left: Int = keep(call_right)\n",
            "let call_right: Int = call_left\n",
            "let shadowed: Int = 0\n",
            "let target: Int = shadowed\n",
            "fn scoped() -> Int\n",
            "  begin\n",
            "    let target: Int = 1\n",
            "    target\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let mut module = lower_surface_ast(&parsed.tree);
    let scoped_initializer = module
        .functions
        .iter()
        .find(|function| function.name.as_deref() == Some("scoped"))
        .and_then(|function| function.body.first())
        .and_then(|line| match &line.kind {
            veln_ast::BodyLineKind::Expr { expr } => Some(expr.clone()),
            _ => None,
        })
        .expect("scoped begin expression");
    module.module_values[4].initializer = scoped_initializer;

    let diagnostics = analyze_surface_module(&module);
    let cycles = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "value.module_initializer_cycle")
        .collect::<Vec<_>>();

    assert_eq!(cycles.len(), 2, "{diagnostics:#?}");
    assert_eq!(
        cycles
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        vec![
            "module value reference `call_left` closes an initializer cycle",
            "module value reference `operator_left` closes an initializer cycle",
        ]
    );
    assert!(
        cycles
            .iter()
            .all(|diagnostic| !diagnostic.message.contains("target")),
        "{diagnostics:#?}"
    );
}

#[test]
fn unsupported_acyclic_dependencies_are_quarantined_transitively() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let base: Int = 1\n",
            "let invalid: Int = base + 1\n",
            "let dependent: Int = invalid\n",
            "let independent: Int = 1\n",
            "fn reads_dependent() -> String\n",
            "  dependent\n",
            "end\n",
            "fn reads_independent() -> String\n",
            "  independent\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.mismatch")
        .collect::<Vec<_>>();

    assert_eq!(mismatches.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        mismatches[0].span.as_ref().map(|span| span.start.line),
        Some(9)
    );
}

#[test]
fn invalid_module_value_dependencies_are_withheld_without_hiding_independent_facts() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn compute() -> Int\n",
            "  1\n",
            "end\n",
            "let invalid: Int = compute()\n",
            "let dependent: Int = invalid\n",
            "let independent: Int = 1\n",
            "fn reads_dependent() -> String\n",
            "  dependent\n",
            "end\n",
            "fn reads_independent() -> String\n",
            "  independent\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.mismatch")
        .collect::<Vec<_>>();

    assert_eq!(mismatches.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        mismatches[0].span.as_ref().map(|span| span.start.line),
        Some(11)
    );
}
