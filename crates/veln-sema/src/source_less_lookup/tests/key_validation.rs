use super::*;

#[test]
fn unconsumable_standard_symbol_key_blocks_lookup_publication() {
    let result = build_source_less_lookup_registries(provider_set(
        UNCONSUMABLE_STANDARD_SYMBOL_KEY,
        &[],
        PRELUDE_MODULE,
        PRELUDE_BUILTIN_MODULE,
        BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        vec![valid_adt_descriptor()],
    ));
    let failure = result.expect_err("unconsumable runtime key blocks publication");

    assert_eq!(failure.code(), "toolchain.invalid_symbol_case");
    assert_eq!(failure.provider, "runtime");
    assert_eq!(failure.name, "foo::bar::print");
    assert_eq!(failure.name_class, SourceLessNameClass::Function);
    assert_eq!(failure.required_initial(), "ascii_lowercase");
    assert_eq!(
        failure.reason,
        InvalidStandardSymbolReason::InvalidLookupKey
    );
    assert_eq!(
        failure.diagnostic().message,
        "compiler-provided function `foo::bar::print` from `runtime` has an invalid source lookup key"
    );
}

#[test]
fn source_less_runtime_leaf_must_be_one_source_identifier_segment() {
    assert_invalid_lookup_key(
        build_source_less_lookup_registries(provider_set(
            RUNTIME_SYMBOL_WITH_QUALIFIED_LEAF,
            &[],
            PRELUDE_MODULE,
            PRELUDE_BUILTIN_MODULE,
            BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
            vec![valid_adt_descriptor()],
        )),
        "runtime",
        "bar::baz",
        SourceLessNameClass::Function,
    );
    assert_invalid_lookup_key(
        build_source_less_lookup_registries(provider_set(
            RUNTIME_SYMBOL_WITH_NON_IDENTIFIER_LEAF,
            &[],
            PRELUDE_MODULE,
            PRELUDE_BUILTIN_MODULE,
            BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
            vec![valid_adt_descriptor()],
        )),
        "runtime",
        "bar-baz",
        SourceLessNameClass::Function,
    );
}

#[test]
fn source_less_prelude_and_adapter_leaves_must_be_source_identifier_segments() {
    assert_invalid_lookup_key(
        build_source_less_lookup_registries(SourceLessLookupProviderSet {
            qualified: VALID_STANDARD_SYMBOLS,
            compatibility_prelude: PRELUDE_SYMBOL_WITH_QUALIFIED_LEAF,
            self_hosting_prelude: &[],
            compiler_adapters: &[],
            standard_module: PRELUDE_MODULE,
            prelude_builtin_module: PRELUDE_BUILTIN_MODULE,
            builtin_type_syntax: BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
            builtin_adts: vec![valid_adt_descriptor()],
        }),
        "prelude",
        "foo::bar",
        SourceLessNameClass::Function,
    );
    assert_invalid_lookup_key(
        build_source_less_lookup_registries(provider_set(
            VALID_STANDARD_SYMBOLS,
            COMPILER_ADAPTER_WITH_NON_IDENTIFIER_LEAF,
            PRELUDE_MODULE,
            PRELUDE_BUILTIN_MODULE,
            BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
            vec![valid_adt_descriptor()],
        )),
        "compiler_adapter",
        "byte-decode",
        SourceLessNameClass::Function,
    );
}

#[test]
fn source_less_bare_prelude_leaf_must_reach_parser_name_lookup() {
    assert_invalid_lookup_key(
        build_source_less_lookup_registries(SourceLessLookupProviderSet {
            qualified: VALID_STANDARD_SYMBOLS,
            compatibility_prelude: PRELUDE_SYMBOL_WITH_CONTEXTUAL_LITERAL_LEAF,
            self_hosting_prelude: &[],
            compiler_adapters: &[],
            standard_module: PRELUDE_MODULE,
            prelude_builtin_module: PRELUDE_BUILTIN_MODULE,
            builtin_type_syntax: BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
            builtin_adts: vec![valid_adt_descriptor()],
        }),
        "prelude",
        "true",
        SourceLessNameClass::Function,
    );
}

#[test]
fn source_less_public_adapter_leaf_must_reach_parser_name_lookup() {
    assert_invalid_lookup_key(
        build_source_less_lookup_registries(provider_set(
            VALID_STANDARD_SYMBOLS,
            PUBLIC_COMPILER_ADAPTER_WITH_CONTEXTUAL_LITERAL_LEAF,
            PRELUDE_MODULE,
            PRELUDE_BUILTIN_MODULE,
            BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
            vec![valid_adt_descriptor()],
        )),
        "compiler_adapter",
        "false",
        SourceLessNameClass::Function,
    );
}

#[test]
fn parser_unreachable_bare_prelude_key_blocks_other_provider_publication() {
    let provider_set = SourceLessLookupProviderSet {
        qualified: VALID_STANDARD_SYMBOLS,
        compatibility_prelude: PRELUDE_SYMBOL_WITH_CONTEXTUAL_LITERAL_LEAF,
        self_hosting_prelude: &[],
        compiler_adapters: &[],
        standard_module: PRELUDE_MODULE,
        prelude_builtin_module: PRELUDE_BUILTIN_MODULE,
        builtin_type_syntax: BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        builtin_adts: vec![valid_adt_descriptor()],
    };

    with_source_less_lookup_provider_set_for_test(provider_set, || {
        let result = with_builtin_adt_registry(|registry| registry.descriptors().len());

        assert!(
            result.is_err(),
            "published ADT lookup must not be available when prelude leaf is not parser-reachable"
        );
    });
}

#[test]
fn source_less_type_syntax_leaf_must_be_one_source_identifier_segment() {
    assert_invalid_lookup_key(
        build_source_less_lookup_registries(provider_set(
            VALID_STANDARD_SYMBOLS,
            &[],
            PRELUDE_MODULE,
            PRELUDE_BUILTIN_MODULE,
            TYPE_SYNTAX_WITH_QUALIFIED_LEAF,
            vec![valid_adt_descriptor()],
        )),
        "type_syntax",
        "Some::Inner",
        SourceLessNameClass::Type,
    );
    assert_invalid_lookup_key(
        build_source_less_lookup_registries(provider_set(
            VALID_STANDARD_SYMBOLS,
            &[],
            PRELUDE_MODULE,
            PRELUDE_BUILTIN_MODULE,
            TYPE_SYNTAX_WITH_NON_IDENTIFIER_LEAF,
            vec![valid_adt_descriptor()],
        )),
        "type_syntax",
        "Some-Inner",
        SourceLessNameClass::Type,
    );
}

#[test]
fn source_less_adt_type_leaf_must_be_one_source_identifier_segment() {
    assert_invalid_lookup_key(
        build_source_less_lookup_registries(provider_set(
            VALID_STANDARD_SYMBOLS,
            &[],
            PRELUDE_MODULE,
            PRELUDE_BUILTIN_MODULE,
            BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
            vec![adt_with_type_name("Some::Inner")],
        )),
        "adt",
        "Some::Inner",
        SourceLessNameClass::Type,
    );
    assert_invalid_lookup_key(
        build_source_less_lookup_registries(provider_set(
            VALID_STANDARD_SYMBOLS,
            &[],
            PRELUDE_MODULE,
            PRELUDE_BUILTIN_MODULE,
            BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
            vec![adt_with_type_name("Some-Inner")],
        )),
        "adt",
        "Some-Inner",
        SourceLessNameClass::Type,
    );
}

#[test]
fn source_less_adt_constructor_leaf_must_be_one_source_identifier_segment() {
    assert_invalid_lookup_key(
        build_source_less_lookup_registries(provider_set(
            VALID_STANDARD_SYMBOLS,
            &[],
            PRELUDE_MODULE,
            PRELUDE_BUILTIN_MODULE,
            BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
            vec![adt_with_constructor_name("Some::Inner")],
        )),
        "adt",
        "Some::Inner",
        SourceLessNameClass::Constructor,
    );
    assert_invalid_lookup_key(
        build_source_less_lookup_registries(provider_set(
            VALID_STANDARD_SYMBOLS,
            &[],
            PRELUDE_MODULE,
            PRELUDE_BUILTIN_MODULE,
            BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
            vec![adt_with_constructor_name("Some-Inner")],
        )),
        "adt",
        "Some-Inner",
        SourceLessNameClass::Constructor,
    );
}

#[test]
fn invalid_runtime_leaf_blocks_other_provider_lookup_publication() {
    let provider_set = SourceLessLookupProviderSet {
        qualified: RUNTIME_SYMBOL_WITH_NON_IDENTIFIER_LEAF,
        compatibility_prelude: &[],
        self_hosting_prelude: &[],
        compiler_adapters: &[],
        standard_module: PRELUDE_MODULE,
        prelude_builtin_module: PRELUDE_BUILTIN_MODULE,
        builtin_type_syntax: BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        builtin_adts: vec![valid_adt_descriptor()],
    };

    with_source_less_lookup_provider_set_for_test(provider_set, || {
        let result = with_builtin_adt_registry(|registry| registry.descriptors().len());

        assert!(
            result.is_err(),
            "published ADT lookup must not be available when runtime leaf validation fails"
        );
    });
}
