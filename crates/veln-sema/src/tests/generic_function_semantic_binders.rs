use super::*;
use crate::semantic_model::Type;
use crate::types::TypeEnvironment;

fn analyze(source_text: &str) -> (SurfaceModule, Vec<Diagnostic>) {
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    (module, diagnostics)
}

fn named_identity(ty: &Type) -> &str {
    let Type::Named { identity, .. } = ty else {
        panic!("expected named type, found {ty:?}");
    };
    identity
}

fn wide_generic_annotation_source(
    binder_count: usize,
    annotation_count: usize,
    annotation: &str,
) -> String {
    let binders = (0..binder_count)
        .map(|index| format!("T{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut source = format!("fn wide<{binders}>(value: T0) -> T0\n");
    for index in 0..annotation_count {
        source.push_str(&format!("  let value{index}: {annotation} = value\n"));
    }
    source.push_str("  value\nend\n");
    source
}

#[test]
fn body_annotations_reuse_one_declaration_type_parameter_context() {
    fn context_work(
        binder_count: usize,
        annotation_count: usize,
        annotation: &str,
    ) -> ((usize, usize), Vec<Diagnostic>) {
        let source = SourceFile::new(
            "main.veln",
            wide_generic_annotation_source(binder_count, annotation_count, annotation),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::types::reset_type_parameter_context_work();
        let diagnostics = analyze_surface_module(&module);
        (
            crate::types::take_type_parameter_context_work(),
            diagnostics,
        )
    }

    let (one_annotation, one_diagnostics) = context_work(64, 1, "T0");
    let (many_annotations, many_diagnostics) = context_work(64, 128, "T0");
    assert!(one_diagnostics.is_empty(), "{one_diagnostics:#?}");
    assert!(many_diagnostics.is_empty(), "{many_diagnostics:#?}");
    assert!(
        one_annotation.0 > 0,
        "the metric must observe context builds"
    );
    assert!(
        one_annotation.1 >= 64,
        "the metric must observe declaration binder indexing"
    );
    assert_eq!(
        many_annotations.1, one_annotation.1,
        "additional annotations must not re-index declaration binders"
    );

    let (invalid_annotations, invalid_diagnostics) = context_work(64, 128, "Int<Int>::Some");
    assert!(
        invalid_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "type.invalid_annotation"),
        "expected invalid annotation diagnostics, found ids: {:?}",
        invalid_diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        invalid_annotations.1, one_annotation.1,
        "failed validation must not re-index declaration binders"
    );
}

#[test]
fn function_binders_are_retained_with_declaration_owned_rigid_identities() {
    let (module, diagnostics) = analyze(concat!(
        "fn first<T>(value: T) -> output: T\n",
        "ensure output == value\n",
        "  let local: T = value\n",
        "  begin\n",
        "    let nested: T = local\n",
        "    nested\n",
        "  end\n",
        "end\n",
        "fn second<T>(value: T) -> T\n",
        "  value\n",
        "end\n",
    ));
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let environment = TypeEnvironment::from_module(&module);
    let first = environment.function("first").expect("first signature");
    let second = environment.function("second").expect("second signature");

    assert_eq!(first.type_parameters.len(), 1);
    assert_eq!(first.type_parameters[0].name, "T");
    assert_eq!(first.type_parameters[0].ordinal, 0);
    assert_eq!(
        first.type_parameters[0].identity,
        named_identity(&first.params[0])
    );
    assert_eq!(
        named_identity(&first.params[0]),
        named_identity(&first.return_type)
    );
    assert_eq!(second.type_parameters.len(), 1);
    assert_ne!(
        first.type_parameters[0].identity,
        second.type_parameters[0].identity
    );
}

#[test]
fn binder_scope_ends_at_the_owning_declaration() {
    let (_, diagnostics) = analyze(concat!(
        "fn owner<T>(value: T) -> T\n",
        "  value\n",
        "end\n",
        "fn outside(value: T) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let unresolved = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved type `T`"
        })
        .collect::<Vec<_>>();
    assert_eq!(unresolved.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        unresolved[0].span.as_ref().map(|span| span.start.line),
        Some(4)
    );
}

#[test]
fn isolated_unbound_annotation_is_unresolved() {
    let (_, diagnostics) = analyze("fn outside(value: T) -> ()\n  ()\nend\n");

    let unresolved = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved type `T`"
        })
        .collect::<Vec<_>>();
    assert_eq!(unresolved.len(), 1, "{diagnostics:#?}");
    let span = unresolved[0].span.as_ref().expect("unresolved span");
    assert_eq!((span.start.line, span.start.column), (1, 19));
}

#[test]
fn visible_schema_body_annotations_are_not_reported_as_unresolved_types() {
    let (_, diagnostics) = analyze(concat!(
        "schema Packet\n",
        "  value: UInt8\n",
        "end\n",
        "fn annotate() -> ()\n",
        "  let packet: Packet = ()\n",
        "  begin\n",
        "    let nested: Packet = packet\n",
        "    ()\n",
        "  end\n",
        "end\n",
    ));

    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "name.unresolved" || diagnostic.message != "unresolved type `Packet`"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn qualified_imported_types_and_aliases_that_match_prelude_names_remain_visible() {
    let module = merged_modules_with_identities(vec![
        (
            "main",
            SourceFile::new(
                "main.veln",
                concat!(
                    "use source_vec\n",
                    "fn accept(values: source_vec::Vec<Int>, table: source_vec::Table<String, Int>) -> source_vec::Dict<String, Int>\n",
                    "  table\n",
                    "end\n",
                ),
            ),
        ),
        (
            "source_vec",
            SourceFile::new(
                "source_vec.veln",
                concat!(
                    "pub type Vec<A>\n",
                    "  pub Wrapped(A)\n",
                    "end\n",
                    "pub type Dict<K, V>\n",
                    "  pub Entry(K, V)\n",
                    "end\n",
                    "pub type Table = Dict\n",
                ),
            ),
        ),
    ]);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn qualified_annotation_selection_checks_the_imported_leaf() {
    let module = merged_modules_with_identities(vec![
        (
            "api",
            SourceFile::new(
                "api.veln",
                concat!(
                    "pub type Present<A>\n",
                    "  pub Present(A)\n",
                    "end\n",
                    "pub type PresentAlias = Present\n",
                    "type Private\n",
                    "  Private\n",
                    "end\n",
                ),
            ),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use api\n",
                    "fn accepts(value: api::Present<Int>, alias: api::PresentAlias<Int>) -> ()\n",
                    "  ()\n",
                    "end\n",
                    "fn missing(value: api::Missing) -> ()\n",
                    "  ()\n",
                    "end\n",
                    "fn private(value: api::Private) -> ()\n",
                    "  ()\n",
                    "end\n",
                ),
            ),
        ),
    ]);
    let environment = TypeEnvironment::from_module(&module);
    let accepts = environment.function("accepts").expect("accepts signature");
    let Type::Named {
        identity: direct_identity,
        args: direct_args,
        ..
    } = &accepts.params[0]
    else {
        panic!(
            "expected imported generic type, found {:?}",
            accepts.params[0]
        );
    };
    let Type::Named {
        identity: alias_identity,
        args: alias_args,
        ..
    } = &accepts.params[1]
    else {
        panic!(
            "expected imported generic alias, found {:?}",
            accepts.params[1]
        );
    };
    assert_eq!(alias_identity, direct_identity);
    assert_eq!(direct_args, &[Type::int()]);
    assert_eq!(alias_args, &[Type::int()]);

    let diagnostics = analyze_surface_module(&module);

    let unresolved = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.unresolved")
        .map(|diagnostic| {
            (
                diagnostic.message.as_str(),
                diagnostic
                    .span
                    .as_ref()
                    .map(|span| (span.start.line, span.start.column)),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        unresolved,
        [
            ("unresolved type `api::Missing`", Some((5, 24))),
            ("unresolved type `api::Private`", Some((8, 24)))
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn standard_package_annotations_use_the_same_resolution_facts() {
    fn diagnostics(include_unrelated_module: bool) -> Vec<Diagnostic> {
        let source = SourceFile::new(
            "standard.veln",
            concat!(
                "fn known(value: Int, optional: Option<Int>, bytes: ByteChunk, step: DecodeStep<Int>) -> Int\n",
                "  value\n",
                "end\n",
                "fn unknown(value: T) -> ()\n",
                "  ()\n",
                "end\n",
            ),
        );
        let mut module = lower_standard_package_callables(&source);
        for function in &mut module.functions {
            function.module_name = Some("std::subject".to_string());
        }
        if include_unrelated_module {
            let unrelated_source = SourceFile::new(
                "unrelated.veln",
                "fn owner<T>(value: T) -> T\n  value\nend\n",
            );
            let mut unrelated = lower_standard_package_callables(&unrelated_source);
            for function in &mut unrelated.functions {
                function.module_name = Some("std::unrelated".to_string());
            }
            module.functions.extend(unrelated.functions);
        }
        analyze_surface_module(&module)
    }

    let baseline_diagnostics = diagnostics(false);

    let unresolved = baseline_diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.unresolved")
        .collect::<Vec<_>>();
    assert_eq!(unresolved.len(), 1, "{baseline_diagnostics:#?}");
    assert_eq!(unresolved[0].message, "unresolved type `T`");
    assert_eq!(
        unresolved[0].span.as_ref().map(|span| span.file.as_str()),
        Some("standard.veln")
    );
    let with_unrelated = diagnostics(true)
        .into_iter()
        .find(|diagnostic| diagnostic.id == "name.unresolved")
        .expect("unresolved standard-package annotation");
    assert_eq!(
        veln_diagnostics::diagnostic_to_json(unresolved[0]).to_json(),
        veln_diagnostics::diagnostic_to_json(&with_unrelated).to_json(),
        "an unrelated standard-package module must not change the diagnostic"
    );
}

#[test]
fn standard_signature_compatibility_is_limited_to_known_functions() {
    let source = SourceFile::new(
        "standard.veln",
        concat!(
            "fn vec_len(value: A) -> A\n",
            "  value\n",
            "end\n",
            "fn unrelated(value: A) -> A\n",
            "  value\n",
            "end\n",
        ),
    );
    let mut module = lower_standard_package_callables(&source);
    for function in &mut module.functions {
        function.module_name = Some("std::prelude".to_string());
    }

    let diagnostics = analyze_surface_module(&module);
    let unresolved = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.unresolved")
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        unresolved,
        ["unresolved type `A`", "unresolved type `A`"],
        "{diagnostics:#?}"
    );
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
