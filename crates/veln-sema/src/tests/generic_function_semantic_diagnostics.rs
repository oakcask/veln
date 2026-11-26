use super::*;

fn analyze(source_text: &str) -> (SurfaceModule, Vec<Diagnostic>) {
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    (module, diagnostics)
}

fn annotation_path_classification_source(count: usize, annotation: &str) -> String {
    let mut source = String::from("fn classify(value: Option<Int>) -> ()\n");
    for index in 0..count {
        source.push_str(&format!("  let value{index}: {annotation} = value\n"));
    }
    source.push_str("  ()\nend\n");
    source
}

#[test]
fn annotation_path_classification_work_has_a_linear_bound() {
    fn work(count: usize, annotation: &str) -> ((usize, usize), Vec<Diagnostic>) {
        let source = SourceFile::new(
            "main.veln",
            annotation_path_classification_source(count, annotation),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::pipeline::reset_annotation_path_classification_work();
        let diagnostics = analyze_surface_module(&module);
        (
            crate::pipeline::take_annotation_path_classification_work(),
            diagnostics,
        )
    }

    const SMALL: usize = 32;
    const LARGE: usize = 64;
    for (label, annotation) in [
        ("valid", "Option<Int>"),
        ("unresolved", "Missing"),
        ("refinement", "Option<Int>::Some"),
    ] {
        let (small, small_diagnostics) = work(SMALL, annotation);
        let (large, large_diagnostics) = work(LARGE, annotation);
        let small_total = small.0 + small.1;
        let large_total = large.0 + large.1;
        eprintln!(
            "annotation path classification: workload={label} count={SMALL}/{LARGE} work={small_total}/{large_total} paths={}/{}, refinements={}/{}",
            small.0, large.0, small.1, large.1
        );
        assert!(large_total > small_total, "{label}: {small:?} {large:?}");
        assert!(
            large_total <= 2 * small_total + 8,
            "doubling {label} annotations must add only linear classification work: {small:?} -> {large:?}"
        );

        let small_unresolved = small_diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.unresolved")
            .count();
        let large_unresolved = large_diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.unresolved")
            .count();
        if label == "unresolved" {
            assert_eq!((small_unresolved, large_unresolved), (SMALL, LARGE));
        } else {
            assert_eq!((small_unresolved, large_unresolved), (0, 0));
        }
    }
}

#[test]
fn unimported_binder_spelling_does_not_change_unbound_annotation_diagnostic() {
    fn diagnostics(include_unimported_owner: bool) -> Vec<Diagnostic> {
        let mut sources = vec![(
            "app",
            SourceFile::new("app.veln", "fn outside(value: T) -> ()\n  ()\nend\n"),
        )];
        if include_unimported_owner {
            sources.push((
                "owner",
                SourceFile::new("owner.veln", "fn owner<T>(value: T) -> T\n  value\nend\n"),
            ));
        }
        analyze_surface_module(&merged_modules_with_identities(sources))
            .into_iter()
            .filter(|diagnostic| diagnostic.id == "name.unresolved")
            .collect()
    }

    let isolated = diagnostics(false);
    let with_unimported_owner = diagnostics(true);
    assert_eq!(isolated.len(), 1, "{isolated:#?}");
    assert_eq!(
        veln_diagnostics::diagnostic_to_json(&isolated[0]).to_json(),
        veln_diagnostics::diagnostic_to_json(&with_unimported_owner[0]).to_json(),
        "an inaccessible declaration must not change another module's diagnostic"
    );
}

#[test]
fn rigid_binder_rejects_a_concrete_body_result() {
    let (_, diagnostics) = analyze(concat!("fn invalid<T>(value: T) -> T\n", "  0\n", "end\n",));

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic.message.contains("expected `T`")
                && diagnostic.message.contains("found `Int`")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn rigid_body_and_contract_operations_are_not_specialized_by_concrete_callers() {
    let (_, diagnostics) = analyze(concat!(
        "fn invalid_body<T>(value: T) -> T\n",
        "  value + 1\n",
        "end\n",
        "fn invalid_contract<T>(value: T) -> result: T\n",
        "ensure result % 2 == 0\n",
        "  value\n",
        "end\n",
        "fn concrete_caller() -> Int\n",
        "  invalid_body(1)\n",
        "end\n",
    ));

    let rigid_operand_mismatches = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic.message == "expected `Int`, but found `T`"
                && diagnostic
                    .details
                    .to_json()
                    .contains("\"constraint\":\"operator_operand\"")
        })
        .collect::<Vec<_>>();
    assert_eq!(rigid_operand_mismatches.len(), 2, "{diagnostics:#?}");
    assert_eq!(
        rigid_operand_mismatches
            .iter()
            .map(|diagnostic| diagnostic.span.as_ref().map(|span| span.start.line))
            .collect::<Vec<_>>(),
        [Some(2), Some(5)]
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic.span.as_ref().map(|span| span.start.line) == Some(9)
                && diagnostic.message == "expected `T`, but found `Int`"
        }),
        "the still-unsupported generic call must not specialize the definition: {diagnostics:#?}"
    );
}

#[test]
fn generic_calls_remain_outside_this_slice() {
    let (_, diagnostics) = analyze(concat!(
        "fn identity<T>(value: T) -> T\n",
        "  value\n",
        "end\n",
        "fn use_identity() -> Int\n",
        "  identity(1)\n",
        "end\n",
    ));

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic.message.contains("expected `T`")
                && diagnostic.message.contains("found `Int`")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn binders_cannot_shadow_builtin_or_source_types() {
    let (_, diagnostics) = analyze(concat!(
        "type Item\n",
        "  Item\n",
        "end\n",
        "fn builtin<Int>(value: Int) -> Int\n",
        "  value\n",
        "end\n",
        "fn source<Item>(value: Item) -> Item\n",
        "  value\n",
        "end\n",
    ));

    let shadows = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.type_parameter_shadow")
        .collect::<Vec<_>>();
    assert_eq!(shadows.len(), 2, "{diagnostics:#?}");
    assert!(shadows[0].related.is_empty(), "{:#?}", shadows[0]);
    assert!(shadows[1].related.iter().any(|related| {
        related
            .to_json()
            .contains("\"kind\":\"shadowed_type_declaration\"")
    }));
    assert!(
        shadows
            .iter()
            .all(|diagnostic| { diagnostic.details.to_json().contains("\"ordinal\":0") })
    );
}

#[test]
fn binder_cannot_shadow_an_imported_type_alias() {
    let module = merged_modules_with_identities(vec![
        (
            "api",
            SourceFile::new(
                "api.veln",
                concat!(
                    "pub type State\n",
                    "  pub Ready\n",
                    "end\n",
                    "pub type Item = State\n",
                ),
            ),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use api\n",
                    "fn shadow<Item>(value: Item) -> Item\n",
                    "  value\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);
    let shadow = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "name.type_parameter_shadow")
        .expect("imported alias shadow diagnostic");
    assert_eq!(
        shadow.span.as_ref().map(|span| span.file.as_str()),
        Some("app.veln")
    );
    assert!(shadow.related.iter().any(|related| {
        let json = related.to_json();
        json.contains("\"kind\":\"shadowed_type_declaration\"")
            && json.contains("\"file\":\"api.veln\"")
            && json.contains("\"line\":4")
    }));
}
