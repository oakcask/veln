use super::*;

#[test]
fn prelude_signature_is_gated_by_standard_symbol_descriptor() {
    let (params, return_type) =
        prelude_signature("vec_len", None).expect("standard helper signature");

    assert_eq!(params, vec![Type::vec(Type::Unknown)]);
    assert_eq!(return_type, Type::int());
    assert!(prelude_signature("unknown_helper", None).is_none());
}

#[test]
fn dictionary_prelude_signatures_are_first_order() {
    let expected_dict = Type::dict(Type::string(), Type::int());
    let expected_option = adt::option_type(Type::int());

    for (name, expected, params, return_type) in [
        (
            "dict_get",
            expected_option,
            vec![expected_dict.clone(), Type::string()],
            adt::option_type(Type::int()),
        ),
        (
            "dict_contains",
            Type::bool(),
            vec![expected_dict.clone(), Type::string()],
            Type::bool(),
        ),
        (
            "dict_insert",
            expected_dict.clone(),
            vec![expected_dict.clone(), Type::string(), Type::int()],
            expected_dict.clone(),
        ),
        (
            "dict_remove",
            expected_dict.clone(),
            vec![expected_dict.clone(), Type::string()],
            expected_dict.clone(),
        ),
    ] {
        let (actual_params, actual_return_type) =
            prelude_signature_with_input(name, Some(&expected), Some(&expected_dict))
                .expect("dictionary helper signature");

        assert_eq!(actual_params, params, "{name}");
        assert_eq!(actual_return_type, return_type, "{name}");
        assert!(
            actual_params
                .iter()
                .all(|param| !matches!(param, Type::Function { .. })),
            "{name} should not be treated as a callback helper"
        );
    }
}

#[test]
fn dictionary_map_signature_preserves_input_and_result_shapes() {
    let input = Type::dict(Type::string(), Type::int());
    let mapped = Type::dict(Type::string(), Type::bool());
    assert_dictionary_callback_signature(
        "dict_map",
        mapped.clone(),
        input.clone(),
        vec![
            input,
            Type::function(vec![Type::string(), Type::int()], Type::bool(), Vec::new()),
        ],
        mapped,
    );
}

#[test]
fn dictionary_filter_with_signature_preserves_context_shape() {
    let input = Type::dict(Type::string(), Type::int());
    assert_dictionary_callback_signature(
        "dict_filter_with",
        input.clone(),
        input.clone(),
        vec![
            Type::Unknown,
            input.clone(),
            Type::function(
                vec![Type::Unknown, Type::string(), Type::int()],
                Type::bool(),
                Vec::new(),
            ),
        ],
        input,
    );
}

#[test]
fn dictionary_fold_signature_preserves_accumulator_shape() {
    let input = Type::dict(Type::string(), Type::int());
    assert_dictionary_callback_signature(
        "dict_fold",
        Type::string(),
        input.clone(),
        vec![
            input,
            Type::string(),
            Type::function(
                vec![Type::string(), Type::string(), Type::int()],
                Type::string(),
                Vec::new(),
            ),
        ],
        Type::string(),
    );
}

#[test]
fn dictionary_try_map_with_signature_preserves_context_and_result_shapes() {
    let input = Type::dict(Type::string(), Type::int());
    let mapped = Type::dict(Type::string(), Type::bool());
    let expected = adt::result_type(mapped, Type::int());
    assert_dictionary_callback_signature(
        "dict_try_map_with",
        expected.clone(),
        input.clone(),
        vec![
            Type::Unknown,
            input,
            Type::function(
                vec![Type::Unknown, Type::string(), Type::int()],
                adt::result_type(Type::bool(), Type::int()),
                Vec::new(),
            ),
        ],
        expected,
    );
}

fn assert_dictionary_callback_signature(
    name: &str,
    expected: Type,
    input: Type,
    expected_params: Vec<Type>,
    expected_return_type: Type,
) {
    let (params, return_type) = prelude_signature_with_input(name, Some(&expected), Some(&input))
        .expect("dictionary callback signature");

    assert_eq!(params, expected_params, "{name}");
    assert_eq!(return_type, expected_return_type, "{name}");
}

#[test]
fn core_dictionary_callback_signatures_preserve_context_and_result_shapes() {
    let mapped = CoreType::dict(CoreType::string(), CoreType::bool());
    let expected = CoreType::result(mapped.clone(), CoreType::int());
    let (_, params, return_type) = core_prelude_signature("dict_try_map_with", Some(&expected))
        .expect("core dictionary callback signature");

    assert_eq!(
        params,
        vec![
            CoreType::Unknown,
            CoreType::dict(CoreType::string(), CoreType::Unknown),
            CoreType::Function {
                params: vec![CoreType::Unknown, CoreType::string(), CoreType::Unknown],
                variadic: None,
                return_type: Box::new(CoreType::result(CoreType::bool(), CoreType::int())),
                effects: Vec::new(),
            },
        ]
    );
    assert_eq!(return_type, expected);
}

#[test]
fn compiler_adapter_fallback_uses_concrete_callback_parameter() {
    let signatures = source_prelude_callback_signatures_from_text(
        "prelude.veln",
        concat!(
            "pub fn future_apply(value: Int, callback: fn(Int, String) -> Bool) -> Bool\n",
            "  callback(value, \"ok\")\n",
            "end\n",
        ),
    );

    let signature = signatures
        .iter()
        .find(|signature| signature.name == "future_apply")
        .expect("future source-backed callback helper should have a fallback signature");

    assert_eq!(
        signature.params,
        vec![
            Type::int(),
            Type::function(vec![Type::int(), Type::string()], Type::bool(), Vec::new())
        ]
    );
    assert_eq!(signature.return_type, Type::bool());
}

#[test]
fn compiler_adapter_fallback_rejects_non_concrete_callback_parameter() {
    let signatures = source_prelude_callback_signatures_from_text(
        "prelude.veln",
        concat!(
            "pub fn future_generic(value: A, callback: fn(A, Int) -> Bool) -> Bool\n",
            "  callback(value, 1)\n",
            "end\n",
        ),
    );

    assert!(
        signatures
            .iter()
            .all(|signature| signature.name != "future_generic"),
        "generic callback parameter should stay outside the fallback"
    );
}
