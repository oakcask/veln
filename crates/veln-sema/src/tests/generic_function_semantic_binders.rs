use super::*;
use crate::semantic_model::Type;
use crate::type_relations::is_assignable;
use crate::types::TypeEnvironment;
use std::collections::HashSet;

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
        "pub fn first<T>(value: T) -> output: T\n",
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
    assert_eq!(first.visibility, veln_ast::Visibility::Public);
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
    assert_eq!(second.visibility, veln_ast::Visibility::Private);
    assert_ne!(
        first.type_parameters[0].identity,
        second.type_parameters[0].identity
    );
}

#[test]
fn rigid_parameter_owner_keys_preserve_option_and_component_boundaries() {
    fn identities(package_name: Option<&str>, module_name: Option<&str>) -> (String, String) {
        let source = SourceFile::new("owner.veln", "fn owner<T>(value: T) -> T\n  value\nend\n");
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let mut module = lower_surface_ast(&parsed.tree);
        let function = &mut module.functions[0];
        function.package_name = package_name.map(str::to_string);
        function.module_name = module_name.map(str::to_string);

        let signature = crate::types::TypeParameterIdentityContext::function_parameters(function);
        let annotations = crate::types::TypeParameterIdentityContext::for_function(function);
        (
            signature[0].identity.clone(),
            annotations
                .identity("T")
                .expect("annotation binder identity"),
        )
    }

    for (left_owner, right_owner) in [
        (
            (None, Some("subject")),
            (Some("workspace"), Some("subject")),
        ),
        (
            (Some("pkg:api"), Some("part")),
            (Some("pkg"), Some("api:part")),
        ),
    ] {
        let left = identities(left_owner.0, left_owner.1);
        let right = identities(right_owner.0, right_owner.1);
        assert_eq!(left.0, left.1);
        assert_eq!(right.0, right.1);
        assert_ne!(left.0, right.0);

        let left_type = Type::resolved_named("T", left.0, Vec::new());
        let right_type = Type::resolved_named("T", right.0, Vec::new());
        assert!(!is_assignable(&left_type, &right_type));
        assert!(!is_assignable(&right_type, &left_type));
    }
}

#[test]
fn annotation_named_symbol_lookup_work_is_linear_in_symbols_and_occurrences() {
    fn measurement(size: usize) -> ((usize, usize), Vec<Diagnostic>) {
        let mut source = String::new();
        for index in 0..size {
            source.push_str(&format!("pub type Visible{index} = Int\n"));
        }
        source.push_str("fn inspect(value: Int) -> ()\n");
        for index in 0..size {
            source.push_str(&format!(
                "  let resolved{index}: Visible{} = value\n",
                size - 1
            ));
            source.push_str(&format!("  let missing{index}: Missing = value\n"));
        }
        source.push_str("  ()\nend\n");

        let source = SourceFile::new("lookup.veln", source);
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::types::reset_annotation_named_symbol_index_work();
        let diagnostics = analyze_surface_module(&module);
        (
            crate::types::take_annotation_named_symbol_index_work(),
            diagnostics,
        )
    }

    let mut totals = Vec::new();
    for size in [32, 64, 128] {
        let ((index_entries, lookups), diagnostics) = measurement(size);
        let missing = diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.id == "name.unresolved"
                    && diagnostic.message == "unresolved type `Missing`"
            })
            .collect::<Vec<_>>();
        assert_eq!(missing.len(), size, "{diagnostics:#?}");
        assert_eq!(
            missing
                .iter()
                .filter_map(|diagnostic| diagnostic.span.as_ref().map(|span| span.start.line))
                .collect::<HashSet<_>>()
                .len(),
            size,
            "each missing occurrence must retain its own span"
        );
        assert!(index_entries >= size, "index build was not measured");
        assert!(
            lookups >= size * 2,
            "resolved and missing lookups were not measured"
        );
        let total = index_entries + lookups;
        assert!(
            total <= size * 8 + 32,
            "size={size} index_entries={index_entries} lookups={lookups}"
        );
        totals.push(total);
    }
    assert!(
        totals[2] <= totals[1] * 2 + 32,
        "doubling visible symbols and occurrences must remain linear: {totals:?}"
    );
}

#[test]
fn annotation_schema_symbol_lookup_work_is_linear_in_symbols_and_occurrences() {
    fn measurement(size: usize) -> (crate::types::SchemaSymbolIndexWork, Vec<Diagnostic>) {
        let mut source = String::new();
        for index in 0..size {
            source.push_str(&format!(
                "pub schema Packet{index}\n  value: Int\nend\npub schema Alias{index} = Packet{index}\n"
            ));
        }
        source.push_str("fn inspect(value: Int) -> ()\n");
        for index in 0..size {
            source.push_str(&format!("  let missing{index}: Missing = value\n"));
        }
        source.push_str("  ()\nend\n");

        let source = SourceFile::new("schema_lookup.veln", source);
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::types::reset_schema_symbol_index_work();
        let diagnostics = analyze_surface_module(&module);
        (crate::types::take_schema_symbol_index_work(), diagnostics)
    }

    let mut totals = Vec::new();
    for size in [32, 64, 128] {
        let (work, diagnostics) = measurement(size);
        let missing = diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.id == "name.unresolved"
                    && diagnostic.message == "unresolved type `Missing`"
            })
            .collect::<Vec<_>>();
        assert_eq!(missing.len(), size, "{diagnostics:#?}");
        assert_eq!(
            missing
                .iter()
                .filter_map(|diagnostic| diagnostic.span.as_ref().map(|span| span.start.line))
                .collect::<HashSet<_>>()
                .len(),
            size,
            "each missing annotation must retain its own span"
        );
        assert!(
            work.schema_entries >= size,
            "schema index build was not measured"
        );
        assert!(
            work.alias_entries >= size,
            "alias index build was not measured"
        );
        assert!(
            work.schema_lookups >= size,
            "schema probes were not measured"
        );
        assert!(work.alias_lookups >= size, "alias probes were not measured");
        totals.push(work.total());
    }
    for adjacent in totals.windows(2) {
        assert!(
            adjacent[1] <= adjacent[0] * 2 + 32,
            "doubling schemas, aliases, and annotations must remain linear: {totals:?}"
        );
    }
    eprintln!("schema annotation index work for sizes 32/64/128: {totals:?}");
}

#[test]
fn function_binders_retain_declaration_order_in_signature_annotations() {
    let (module, diagnostics) = analyze(concat!(
        "fn ordered<First, Second>(first: First, second: Second) -> Second\n",
        "  second\n",
        "end\n",
    ));
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let environment = TypeEnvironment::from_module(&module);
    let ordered = environment.function("ordered").expect("ordered signature");
    assert_eq!(ordered.type_parameters.len(), 2);
    assert_eq!(ordered.type_parameters[0].name, "First");
    assert_eq!(ordered.type_parameters[0].ordinal, 0);
    assert_eq!(ordered.type_parameters[1].name, "Second");
    assert_eq!(ordered.type_parameters[1].ordinal, 1);
    assert_ne!(
        ordered.type_parameters[0].identity,
        ordered.type_parameters[1].identity
    );
    assert_eq!(
        ordered.type_parameters[0].identity,
        named_identity(&ordered.params[0])
    );
    assert_eq!(
        ordered.type_parameters[1].identity,
        named_identity(&ordered.params[1])
    );
    assert_eq!(
        ordered.type_parameters[1].identity,
        named_identity(&ordered.return_type)
    );
}

#[test]
fn qualified_and_applied_binder_paths_fail_in_signatures_and_body_annotations() {
    let (_, diagnostics) = analyze(concat!(
        "fn qualified_parameter<T>(value: T::Leaf) -> ()\n",
        "  ()\n",
        "end\n",
        "fn qualified_return<T>() -> T::Leaf\n",
        "  ()\n",
        "end\n",
        "fn applied_parameter<T>(value: T<Int>) -> ()\n",
        "  ()\n",
        "end\n",
        "fn applied_return<T>() -> T<Int>\n",
        "  ()\n",
        "end\n",
        "fn invalid_locals<T>(value: T) -> ()\n",
        "  let local_qualified: T::Leaf = value\n",
        "  let local_applied: T<Int> = value\n",
        "  ()\n",
        "end\n",
    ));

    let unresolved = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved type `T::Leaf`"
        })
        .collect::<Vec<_>>();
    assert_eq!(unresolved.len(), 3, "{diagnostics:#?}");
    assert_eq!(
        unresolved
            .iter()
            .map(|diagnostic| diagnostic.span.as_ref().map(|span| span.start.line))
            .collect::<Vec<_>>(),
        [Some(1), Some(4), Some(14)]
    );

    let invalid_applied = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "type.invalid_annotation"
                && diagnostic
                    .message
                    .contains("`T` expects 0 type argument(s), found 1")
        })
        .collect::<Vec<_>>();
    assert_eq!(invalid_applied.len(), 3, "{diagnostics:#?}");
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "name.unresolved" || diagnostic.message != "unresolved type `T`"
        }),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "type.mismatch"),
        "invalid annotations must not report success through recovery: {diagnostics:#?}"
    );
}

#[test]
fn refinement_type_arguments_keep_owner_aware_name_and_arity_checks() {
    let (_, diagnostics) = analyze(concat!(
        "fn outside_signature(value: Option<T>::Some) -> ()\n",
        "  ()\n",
        "end\n",
        "fn outside_body(value: Option<Int>::Some) -> ()\n",
        "  let local: Option<T>::Some = value\n",
        "  ()\n",
        "end\n",
        "fn applied<T>(value: Option<T<Int>>::Some) -> ()\n",
        "  let local: Option<T<Int>>::Some = value\n",
        "  ()\n",
        "end\n",
        "fn valid<T>(value: Option<T>::Some) -> Option<T>::Some\n",
        "  let local: Option<T>::Some = value\n",
        "  local\n",
        "end\n",
    ));

    let unresolved = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved type `T`"
        })
        .collect::<Vec<_>>();
    assert_eq!(unresolved.len(), 2, "{diagnostics:#?}");
    assert_eq!(
        unresolved
            .iter()
            .map(|diagnostic| diagnostic.span.as_ref().map(|span| span.start.line))
            .collect::<Vec<_>>(),
        [Some(1), Some(5)]
    );

    let invalid_applied = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "type.invalid_annotation"
                && diagnostic
                    .message
                    .contains("`T` expects 0 type argument(s), found 1")
        })
        .collect::<Vec<_>>();
    assert_eq!(invalid_applied.len(), 2, "{diagnostics:#?}");
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic
                .span
                .as_ref()
                .is_none_or(|span| span.start.line < 13)
                || !matches!(
                    diagnostic.id.as_str(),
                    "name.unresolved" | "type.invalid_annotation"
                )
        }),
        "the owning declaration must not gain a name or arity diagnostic: {diagnostics:#?}"
    );
}

#[test]
fn rigid_callers_do_not_publish_type_parameters_into_private_signatures() {
    for (source, owner_call_line) in [
        (
            concat!(
                "fn passthrough(value)\n",
                "  value\n",
                "end\n",
                "fn owner<T>(value: T) -> T\n",
                "  passthrough(value)\n",
                "end\n",
                "fn concrete() -> Int\n",
                "  passthrough(1)\n",
                "end\n",
            ),
            5,
        ),
        (
            concat!(
                "fn passthrough(value)\n",
                "  value\n",
                "end\n",
                "fn concrete() -> Int\n",
                "  passthrough(1)\n",
                "end\n",
                "fn owner<T>(value: T) -> T\n",
                "  passthrough(value)\n",
                "end\n",
            ),
            8,
        ),
    ] {
        let (module, diagnostics) = analyze(source);
        assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
        let mismatches = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .collect::<Vec<_>>();
        assert_eq!(mismatches.len(), 2, "{diagnostics:#?}");
        assert_eq!(
            mismatches
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>(),
            [
                "expected `Int`, but found `T`",
                "expected `T`, but found `Int`",
            ]
        );
        assert!(
            mismatches.iter().all(|diagnostic| diagnostic
                .span
                .as_ref()
                .map(|span| span.start.line)
                == Some(owner_call_line)),
            "{diagnostics:#?}"
        );

        let environment = TypeEnvironment::from_module(&module);
        let passthrough = environment
            .function("passthrough")
            .expect("passthrough signature");
        assert_eq!(passthrough.params, [Type::int()]);
        assert_eq!(passthrough.return_type, Type::int());
    }
}

#[test]
fn binder_bearing_functions_require_complete_signatures_before_private_inference() {
    let (module, diagnostics) = analyze(concat!(
        "fn missing_parameter<T>(value) -> T\n",
        "  value\n",
        "end\n",
        "fn missing_return<T>(value: T)\n",
        "  value\n",
        "end\n",
        "pub fn public_missing<T>(value)\n",
        "  value\n",
        "end\n",
    ));

    let generic_missing = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.generic_signature_missing")
        .collect::<Vec<_>>();
    assert_eq!(generic_missing.len(), 2, "{diagnostics:#?}");
    assert!(generic_missing.iter().any(|diagnostic| {
        diagnostic.message == "generic parameter `value` has no type annotation"
            && diagnostic
                .span
                .as_ref()
                .is_some_and(|span| span.start.line == 1)
    }));
    assert!(generic_missing.iter().any(|diagnostic| {
        diagnostic.message == "generic function has no return type annotation"
            && diagnostic
                .span
                .as_ref()
                .is_some_and(|span| span.start.line == 4)
    }));
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.public_signature_missing")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.private_inference_incomplete")
    );

    let environment = TypeEnvironment::from_module(&module);
    let missing_parameter = environment
        .function("missing_parameter")
        .expect("missing-parameter signature");
    assert_eq!(missing_parameter.params, [Type::Unknown]);
    let missing_return = environment
        .function("missing_return")
        .expect("missing-return signature");
    assert_eq!(missing_return.return_type, Type::Unknown);
}

#[test]
fn functions_without_ordinary_binders_keep_private_signature_inference() {
    let (module, diagnostics) = analyze(concat!(
        "fn ordinary(value)\n",
        "  value\n",
        "end\n",
        "fn effect_only<effect E>(value) effects [...E]\n",
        "  value\n",
        "end\n",
        "fn use_ordinary() -> Int\n",
        "  ordinary(1)\n",
        "end\n",
        "fn use_effect_only() -> String\n",
        "  effect_only(\"ok\")\n",
        "end\n",
    ));
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let environment = TypeEnvironment::from_module(&module);
    let ordinary = environment
        .function("ordinary")
        .expect("ordinary signature");
    assert_eq!(ordinary.params, [Type::int()]);
    assert_eq!(ordinary.return_type, Type::int());
    let effect_only = environment
        .function("effect_only")
        .expect("effect-only signature");
    assert_eq!(effect_only.params, [Type::string()]);
    assert_eq!(effect_only.return_type, Type::string());
}

#[test]
fn rigid_function_value_context_does_not_publish_into_a_private_signature() {
    let (module, diagnostics) = analyze(concat!(
        "fn callback(value)\n",
        "  value\n",
        "end\n",
        "fn owner<T>() -> fn(T) -> T\n",
        "  callback\n",
        "end\n",
        "fn concrete() -> fn(Int) -> Int\n",
        "  callback\n",
        "end\n",
        "fn collect(values: ...) -> ()\n",
        "  ()\n",
        "end\n",
        "fn rest_owner<T>() -> fn(...T) -> ()\n",
        "  collect\n",
        "end\n",
        "fn rest_concrete() -> fn(...Int) -> ()\n",
        "  collect\n",
        "end\n",
    ));
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic
                    .span
                    .as_ref()
                    .is_some_and(|span| span.start.line == 5 || span.start.line == 14)
        }),
        "{diagnostics:#?}"
    );

    let environment = TypeEnvironment::from_module(&module);
    let callback = environment
        .function("callback")
        .expect("callback signature");
    assert_eq!(callback.params, [Type::int()]);
    assert_eq!(callback.return_type, Type::int());
    let collect = environment.function("collect").expect("collect signature");
    assert_eq!(collect.variadic, Some(Type::int()));
}

#[test]
fn composite_local_expected_type_does_not_publish_a_foreign_rigid_parameter() {
    let (module, diagnostics) = analyze(concat!(
        "fn empty()\n",
        "  []\n",
        "end\n",
        "fn owner<T>() -> ()\n",
        "  let values: Vec<T> = empty()\n",
        "  ()\n",
        "end\n",
        "fn concrete() -> Vec<Int>\n",
        "  empty()\n",
        "end\n",
    ));
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic
                    .span
                    .as_ref()
                    .is_some_and(|span| span.start.line == 5)
        }),
        "{diagnostics:#?}"
    );

    let environment = TypeEnvironment::from_module(&module);
    let empty = environment.function("empty").expect("empty signature");
    assert_eq!(empty.return_type, Type::vec(Type::int()));
}

#[test]
fn prelude_callback_inference_does_not_publish_a_foreign_rigid_result() {
    let (module, diagnostics) = analyze(concat!(
        "fn items(value: Int)\n",
        "  []\n",
        "end\n",
        "fn owner<T>(values: Vec<Int>) -> Vec<Vec<T>>\n",
        "  vec_map(values, items)\n",
        "end\n",
        "fn concrete(values: Vec<Int>) -> Vec<Vec<String>>\n",
        "  vec_map(values, items)\n",
        "end\n",
    ));
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic
                    .span
                    .as_ref()
                    .is_some_and(|span| span.start.line == 5)
        }),
        "{diagnostics:#?}"
    );

    let environment = TypeEnvironment::from_module(&module);
    let items = environment.function("items").expect("items signature");
    assert_eq!(items.return_type, Type::vec(Type::string()));
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
fn rigid_binders_resolve_before_same_spelled_schemas_in_all_function_annotations() {
    let (_, diagnostics) = analyze(concat!(
        "schema T\n",
        "  value: UInt8\n",
        "end\n",
        "fn retain<T>(value: T) -> T\n",
        "  let local: T = value\n",
        "  local\n",
        "end\n",
    ));

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.schema_reference"),
        "declaration-owned binders must win before schema lookup: {diagnostics:#?}"
    );
}

#[test]
fn function_binder_spellings_do_not_escape_into_adt_payloads() {
    let (_, diagnostics) = analyze(concat!(
        "fn identity<T>(value: T) -> T\n",
        "  value\n",
        "end\n",
        "type Leak\n",
        "  Direct(value: T)\n",
        "  Nested(value: Option<T>)\n",
        "  Record(value: {item: T})\n",
        "end\n",
        "type Owned<T>\n",
        "  Value(value: T)\n",
        "  Next(value: Option<Owned<T>>)\n",
        "end\n",
    ));

    let unresolved = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved type `T`"
        })
        .collect::<Vec<_>>();
    assert_eq!(unresolved.len(), 3, "{diagnostics:#?}");
    assert_eq!(
        unresolved
            .iter()
            .filter_map(|diagnostic| diagnostic.span.as_ref().map(|span| span.start.line))
            .collect::<Vec<_>>(),
        [5, 6, 7]
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "name.unresolved"
                || diagnostic
                    .span
                    .as_ref()
                    .is_none_or(|span| span.start.line < 9)
        }),
        "an ADT's own parameters and self type must stay in scope: {diagnostics:#?}"
    );
}

#[test]
fn known_descriptor_leaves_do_not_resolve_unknown_or_binder_qualifiers() {
    let (_, diagnostics) = analyze(concat!(
        "fn invalid<T>(left: T::Option, right: missing::Option<Int>) -> ()\n",
        "  let local: T::Option = left\n",
        "  ()\n",
        "end\n",
        "fn valid(value: prelude::Option<Int>) -> prelude::Option<Int>\n",
        "  value\n",
        "end\n",
    ));

    let unresolved = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.unresolved")
        .collect::<Vec<_>>();
    assert_eq!(unresolved.len(), 3, "{diagnostics:#?}");
    assert_eq!(
        unresolved
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        [
            "unresolved type `T::Option`",
            "unresolved type `missing::Option`",
            "unresolved type `T::Option`",
        ]
    );
    assert_eq!(
        unresolved
            .iter()
            .filter_map(|diagnostic| diagnostic.span.as_ref().map(|span| span.start.column))
            .collect::<Vec<_>>(),
        [21, 39, 14]
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "name.unresolved" || !diagnostic.message.contains("prelude::Option")
        }),
        "the supported prelude qualifier must remain resolved: {diagnostics:#?}"
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

    let retained_binder_source = SourceFile::new(
        "retained_binder.veln",
        "fn vec_len<T>(value: A) -> A\n  value\nend\n",
    );
    let mut retained_binder_module = lower_standard_package_callables(&retained_binder_source);
    for function in &mut retained_binder_module.functions {
        function.module_name = Some("std::prelude".to_string());
    }
    let retained_binder_diagnostics = analyze_surface_module(&retained_binder_module);
    let unresolved = retained_binder_diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.unresolved")
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        unresolved,
        ["unresolved type `A`", "unresolved type `A`"],
        "{retained_binder_diagnostics:#?}"
    );

    let declared_binder_source = SourceFile::new(
        "declared_binder.veln",
        "fn vec_len<A>(value: A) -> A\n  value\nend\n",
    );
    let mut declared_binder_module = lower_standard_package_callables(&declared_binder_source);
    for function in &mut declared_binder_module.functions {
        function.module_name = Some("std::prelude".to_string());
    }
    let declared_binder_diagnostics = analyze_surface_module(&declared_binder_module);
    assert!(
        declared_binder_diagnostics.is_empty(),
        "{declared_binder_diagnostics:#?}"
    );
}
