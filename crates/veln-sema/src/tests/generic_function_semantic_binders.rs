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
        (
            first.type_parameters[0].span.start.line,
            first.type_parameters[0].span.start.column,
        ),
        (1, 14)
    );
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
    assert_eq!(
        (
            second.type_parameters[0].span.start.line,
            second.type_parameters[0].span.start.column,
        ),
        (9, 11)
    );
    assert_ne!(
        first.type_parameters[0].identity,
        second.type_parameters[0].identity
    );
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
