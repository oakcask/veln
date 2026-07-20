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
