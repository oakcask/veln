use super::*;

#[test]
fn prelude_builtin_lookup_consumes_published_module_key() {
    const ADAPTERS: &[StandardSymbolDescriptor] = &[StandardSymbolDescriptor {
        module: None,
        name: "byte",
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Prelude,
        effects: &[],
        lowering: None,
        signature: None,
        stability: StandardSymbolStability::CompatibilityOnly,
    }];
    let provider_set = SourceLessLookupProviderSet {
        qualified: &[],
        compatibility_prelude: &[],
        self_hosting_prelude: &[],
        compiler_adapters: ADAPTERS,
        standard_module: PRELUDE_MODULE,
        prelude_builtin_module: "intrinsic",
        builtin_type_syntax: BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        builtin_adts: vec![valid_adt_descriptor()],
    };

    with_source_less_lookup_provider_set_for_test(provider_set, || {
        assert!(
            crate::prelude::qualified_prelude_builtin_signature_with_input(
                &path("prelude_builtin", "byte"),
                None,
                None,
            )
            .is_none(),
            "lookup must not use a registry-external prelude_builtin key"
        );
        assert!(
            crate::prelude::qualified_prelude_builtin_signature_with_input(
                &path("intrinsic", "byte"),
                None,
                None,
            )
            .is_some(),
            "lookup should consume the published prelude_builtin key"
        );
    });
}

#[test]
fn prelude_lookup_consumes_published_standard_module_key() {
    const PRELUDE_SYMBOLS: &[StandardSymbolDescriptor] = &[StandardSymbolDescriptor {
        module: None,
        name: "byte",
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Prelude,
        effects: &[],
        lowering: None,
        signature: None,
        stability: StandardSymbolStability::CompatibilityOnly,
    }];
    let provider_set = SourceLessLookupProviderSet {
        qualified: &[],
        compatibility_prelude: PRELUDE_SYMBOLS,
        self_hosting_prelude: &[],
        compiler_adapters: &[],
        standard_module: "stdlib",
        prelude_builtin_module: PRELUDE_BUILTIN_MODULE,
        builtin_type_syntax: BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        builtin_adts: vec![valid_adt_descriptor()],
    };

    with_source_less_lookup_provider_set_for_test(provider_set, || {
        assert!(
            crate::prelude::qualified_prelude_signature_with_input(
                &path("prelude", "byte"),
                None,
                None,
            )
            .is_none(),
            "lookup must not use a registry-external prelude key"
        );
        assert!(
            crate::prelude::qualified_prelude_signature_with_input(
                &path("stdlib", "byte"),
                None,
                None,
            )
            .is_some(),
            "lookup should consume the published prelude key"
        );
    });
}

#[test]
fn core_prelude_lookup_consumes_published_standard_module_key() {
    const PRELUDE_SYMBOLS: &[StandardSymbolDescriptor] = &[StandardSymbolDescriptor {
        module: None,
        name: "byte",
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Prelude,
        effects: &[],
        lowering: None,
        signature: None,
        stability: StandardSymbolStability::CompatibilityOnly,
    }];
    let provider_set = SourceLessLookupProviderSet {
        qualified: &[],
        compatibility_prelude: PRELUDE_SYMBOLS,
        self_hosting_prelude: &[],
        compiler_adapters: &[],
        standard_module: "stdlib",
        prelude_builtin_module: PRELUDE_BUILTIN_MODULE,
        builtin_type_syntax: BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        builtin_adts: vec![valid_adt_descriptor()],
    };

    with_source_less_lookup_provider_set_for_test(provider_set, || {
        assert!(
            crate::prelude::qualified_core_prelude_signature(&path("prelude", "byte"), None)
                .is_none(),
            "core lookup must not use a registry-external prelude key"
        );
        assert!(
            crate::prelude::qualified_core_prelude_signature(&path("stdlib", "byte"), None)
                .is_some(),
            "core lookup should consume the published prelude key"
        );
    });
}

#[test]
fn prelude_effect_lookup_consumes_published_standard_module_key() {
    let provider_set = SourceLessLookupProviderSet {
        qualified: &[],
        compatibility_prelude: &[],
        self_hosting_prelude: &[],
        compiler_adapters: &[],
        standard_module: "stdlib",
        prelude_builtin_module: PRELUDE_BUILTIN_MODULE,
        builtin_type_syntax: BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        builtin_adts: vec![valid_adt_descriptor()],
    };

    with_source_less_lookup_provider_set_for_test(provider_set, || {
        assert!(
            crate::effects::prelude_effects(&path("prelude", "stream_adapter_accept_loop"))
                .is_none(),
            "effect lookup must not use a registry-external prelude key"
        );
        assert_eq!(
            crate::effects::prelude_effects(&path("stdlib", "stream_adapter_accept_loop")),
            Some(&["net", "concurrency"][..]),
            "effect lookup should consume the published prelude key"
        );
    });
}
