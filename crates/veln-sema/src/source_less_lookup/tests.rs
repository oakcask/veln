use veln_ast::{SurfaceModule, Visibility};

use super::*;
use crate::adt::{AdtDescriptor, AdtVariantDescriptor, AdtVariantKind};
use crate::builtin_type_syntax::{BUILTIN_TYPE_SYNTAX_DESCRIPTORS, BuiltinTypeSyntaxDescriptor};
use crate::source_less_names::{InvalidStandardSymbolReason, SourceLessNameClass};
use crate::standard_symbols::{StandardSymbolKind, StandardSymbolStability};

fn path(module: &str, name: &str) -> Vec<String> {
    vec![module.to_string(), name.to_string()]
}

const VALID_STANDARD_SYMBOLS: &[StandardSymbolDescriptor] = &[StandardSymbolDescriptor {
    module: Some("stdio"),
    name: "print",
    name_class: SourceLessNameClass::Function,
    kind: StandardSymbolKind::Runtime,
    effects: &[],
    lowering: Some("runtime.stdio.print"),
    signature: None,
    stability: StandardSymbolStability::RequiredForSelfHosting,
}];

const INVALID_STANDARD_SYMBOLS: &[StandardSymbolDescriptor] = &[StandardSymbolDescriptor {
    module: Some("Std"),
    name: "print",
    name_class: SourceLessNameClass::Function,
    kind: StandardSymbolKind::Runtime,
    effects: &[],
    lowering: Some("runtime.Std.print"),
    signature: None,
    stability: StandardSymbolStability::RequiredForSelfHosting,
}];

const UNCONSUMABLE_STANDARD_SYMBOL_KEY: &[StandardSymbolDescriptor] = &[StandardSymbolDescriptor {
    module: Some("foo::bar"),
    name: "print",
    name_class: SourceLessNameClass::Function,
    kind: StandardSymbolKind::Runtime,
    effects: &[],
    lowering: Some("runtime.foo.bar.print"),
    signature: None,
    stability: StandardSymbolStability::RequiredForSelfHosting,
}];

const RUNTIME_SYMBOL_WITH_QUALIFIED_LEAF: &[StandardSymbolDescriptor] =
    &[StandardSymbolDescriptor {
        module: Some("foo"),
        name: "bar::baz",
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Runtime,
        effects: &[],
        lowering: Some("runtime.foo.bar.baz"),
        signature: None,
        stability: StandardSymbolStability::RequiredForSelfHosting,
    }];

const RUNTIME_SYMBOL_WITH_NON_IDENTIFIER_LEAF: &[StandardSymbolDescriptor] =
    &[StandardSymbolDescriptor {
        module: Some("foo"),
        name: "bar-baz",
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Runtime,
        effects: &[],
        lowering: Some("runtime.foo.bar-baz"),
        signature: None,
        stability: StandardSymbolStability::RequiredForSelfHosting,
    }];

const PRELUDE_SYMBOL_WITH_QUALIFIED_LEAF: &[StandardSymbolDescriptor] =
    &[StandardSymbolDescriptor {
        module: None,
        name: "foo::bar",
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Prelude,
        effects: &[],
        lowering: None,
        signature: None,
        stability: StandardSymbolStability::CompatibilityOnly,
    }];

const COMPILER_ADAPTER_WITH_NON_IDENTIFIER_LEAF: &[StandardSymbolDescriptor] =
    &[StandardSymbolDescriptor {
        module: None,
        name: "byte-decode",
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Prelude,
        effects: &[],
        lowering: None,
        signature: None,
        stability: StandardSymbolStability::CompatibilityOnly,
    }];

const INVALID_TYPE_SYNTAX: &[BuiltinTypeSyntaxDescriptor] = &[BuiltinTypeSyntaxDescriptor {
    name: "result",
    name_class: SourceLessNameClass::Type,
    arity: 2,
}];
const TYPE_SYNTAX_WITH_QUALIFIED_LEAF: &[BuiltinTypeSyntaxDescriptor] =
    &[BuiltinTypeSyntaxDescriptor {
        name: "Some::Inner",
        name_class: SourceLessNameClass::Type,
        arity: 0,
    }];
const TYPE_SYNTAX_WITH_NON_IDENTIFIER_LEAF: &[BuiltinTypeSyntaxDescriptor] =
    &[BuiltinTypeSyntaxDescriptor {
        name: "Some-Inner",
        name_class: SourceLessNameClass::Type,
        arity: 0,
    }];
const DUPLICATE_TYPE_SYNTAX: &[BuiltinTypeSyntaxDescriptor] = &[
    BuiltinTypeSyntaxDescriptor {
        name: "Result",
        name_class: SourceLessNameClass::Type,
        arity: 2,
    },
    BuiltinTypeSyntaxDescriptor {
        name: "Result",
        name_class: SourceLessNameClass::Type,
        arity: 1,
    },
];

fn valid_adt_descriptor() -> AdtDescriptor {
    AdtDescriptor {
        type_name: "Boxed".to_string(),
        name_class: SourceLessNameClass::Type,
        module_name: None,
        type_parameters: Vec::new(),
        variants: vec![AdtVariantDescriptor {
            name: "Boxed".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: Vec::new(),
            coverage_case: "Boxed(_)".to_string(),
            visibility: Visibility::Public,
        }],
        diagnostic_name: "boxed".to_string(),
        propagation: None,
        visibility: Visibility::Public,
    }
}

fn invalid_adt_descriptor() -> AdtDescriptor {
    let mut descriptor = valid_adt_descriptor();
    descriptor.variants[0].name = "boxed".to_string();
    descriptor
}

fn adt_with_type_name(name: &str) -> AdtDescriptor {
    let mut descriptor = valid_adt_descriptor();
    descriptor.type_name = name.to_string();
    descriptor
}

fn adt_with_constructor_name(name: &str) -> AdtDescriptor {
    let mut descriptor = valid_adt_descriptor();
    descriptor.variants[0].name = name.to_string();
    descriptor
}

fn empty_module() -> SurfaceModule {
    SurfaceModule {
        module: None,
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    }
}

fn provider_set(
    qualified: &'static [StandardSymbolDescriptor],
    compiler_adapters: &'static [StandardSymbolDescriptor],
    standard_module: &'static str,
    prelude_builtin_module: &'static str,
    builtin_type_syntax: &'static [BuiltinTypeSyntaxDescriptor],
    builtin_adts: Vec<AdtDescriptor>,
) -> SourceLessLookupProviderSet {
    SourceLessLookupProviderSet {
        qualified,
        compatibility_prelude: &[],
        self_hosting_prelude: &[],
        compiler_adapters,
        standard_module,
        prelude_builtin_module,
        builtin_type_syntax,
        builtin_adts,
    }
}

fn assert_invalid_lookup_key(
    result: Result<SourceLessLookupRegistries, crate::source_less_names::InvalidStandardSymbolCase>,
    provider: &'static str,
    name: &str,
    name_class: SourceLessNameClass,
) {
    let failure = result.expect_err("invalid source lookup segment blocks publication");

    assert_eq!(failure.code(), "toolchain.invalid_symbol_case");
    assert_eq!(failure.provider, provider);
    assert_eq!(failure.name, name);
    assert_eq!(failure.name_class, name_class);
    assert_eq!(
        failure.reason,
        InvalidStandardSymbolReason::InvalidLookupKey
    );
}

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

#[test]
fn unconsumable_standard_symbol_key_blocks_production_lookup() {
    let provider_set = SourceLessLookupProviderSet {
        qualified: UNCONSUMABLE_STANDARD_SYMBOL_KEY,
        compatibility_prelude: &[],
        self_hosting_prelude: &[],
        compiler_adapters: &[],
        standard_module: PRELUDE_MODULE,
        prelude_builtin_module: PRELUDE_BUILTIN_MODULE,
        builtin_type_syntax: BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        builtin_adts: vec![valid_adt_descriptor()],
    };

    with_source_less_lookup_provider_set_for_test(provider_set, || {
        let result = std::panic::catch_unwind(|| {
            crate::source_less_lookup::qualified_symbol(&path("foo", "bar"))
        });

        assert!(
            result.is_err(),
            "production standard-symbol lookup must not publish unconsumable keys"
        );
    });
}

#[test]
fn invalid_adt_descriptor_blocks_standard_symbol_publication() {
    let result = build_source_less_lookup_registries(provider_set(
        VALID_STANDARD_SYMBOLS,
        &[],
        PRELUDE_MODULE,
        PRELUDE_BUILTIN_MODULE,
        BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        vec![invalid_adt_descriptor()],
    ));
    let failure = result.expect_err("invalid ADT blocks all source-less lookup");

    assert_eq!(failure.provider, "adt");
    assert_eq!(failure.name, "boxed");
    assert_eq!(failure.name_class, SourceLessNameClass::Constructor);
    assert_eq!(failure.reason, InvalidStandardSymbolReason::InvalidCase);
}

#[test]
fn invalid_adt_descriptor_blocks_every_lookup_publication() {
    let result = build_source_less_lookup_registries(provider_set(
        VALID_STANDARD_SYMBOLS,
        &[],
        PRELUDE_MODULE,
        PRELUDE_BUILTIN_MODULE,
        BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        vec![invalid_adt_descriptor()],
    ));

    assert!(
        result.is_err(),
        "valid runtime symbols must not publish when ADT lookup validation fails"
    );
}

#[test]
fn invalid_standard_symbol_descriptor_blocks_adt_publication() {
    let result = build_source_less_lookup_registries(provider_set(
        INVALID_STANDARD_SYMBOLS,
        &[],
        PRELUDE_MODULE,
        PRELUDE_BUILTIN_MODULE,
        BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        vec![valid_adt_descriptor()],
    ));
    let failure = result.expect_err("invalid standard symbol blocks all source-less lookup");

    assert_eq!(failure.provider, "runtime");
    assert_eq!(failure.name, "Std");
    assert_eq!(failure.name_class, SourceLessNameClass::Module);
    assert_eq!(failure.reason, InvalidStandardSymbolReason::InvalidCase);
}

#[test]
fn invalid_standard_symbol_descriptor_blocks_every_lookup_publication() {
    let result = build_source_less_lookup_registries(provider_set(
        INVALID_STANDARD_SYMBOLS,
        &[],
        PRELUDE_MODULE,
        PRELUDE_BUILTIN_MODULE,
        BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        vec![valid_adt_descriptor()],
    ));

    assert!(
        result.is_err(),
        "valid built-in ADTs must not publish when standard symbol validation fails"
    );
}

#[test]
fn shared_registry_exposes_lookup_only_after_all_providers_validate() {
    let registries = build_source_less_lookup_registries(provider_set(
        VALID_STANDARD_SYMBOLS,
        &[],
        PRELUDE_MODULE,
        PRELUDE_BUILTIN_MODULE,
        BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        vec![valid_adt_descriptor()],
    ))
    .expect("all providers validate");

    assert!(
        registries
            .standard_symbols
            .qualified_symbol(&path("stdio", "print"))
            .is_some()
    );
    assert_eq!(registries.standard_module, "prelude");
    assert_eq!(registries.prelude_builtin_module, "prelude_builtin");
    assert_eq!(registries.builtin_type_syntax.arity("Result"), Some(2));
    let option = crate::semantic_model::Type::named("Boxed", Vec::new());
    assert!(
        registries
            .builtin_adts
            .descriptor_for_type(&option)
            .is_some()
    );
}

#[test]
fn invalid_adt_descriptor_blocks_production_standard_symbol_lookup() {
    let provider_set = SourceLessLookupProviderSet {
        qualified: VALID_STANDARD_SYMBOLS,
        compatibility_prelude: &[],
        self_hosting_prelude: &[],
        compiler_adapters: &[],
        standard_module: PRELUDE_MODULE,
        prelude_builtin_module: PRELUDE_BUILTIN_MODULE,
        builtin_type_syntax: BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        builtin_adts: vec![invalid_adt_descriptor()],
    };

    with_source_less_lookup_provider_set_for_test(provider_set, || {
        let result = std::panic::catch_unwind(|| {
            crate::source_less_lookup::qualified_symbol(&path("stdio", "print"))
        });

        assert!(
            result.is_err(),
            "production standard-symbol lookup must not publish when ADT validation fails"
        );
    });
}

#[test]
fn invalid_standard_symbol_descriptor_blocks_production_adt_lookup() {
    let provider_set = SourceLessLookupProviderSet {
        qualified: INVALID_STANDARD_SYMBOLS,
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
            "published ADT lookup must not be available when standard-symbol validation fails"
        );
    });
}

#[test]
fn published_adt_registry_seeds_application_lookup() {
    let registries = build_source_less_lookup_registries(provider_set(
        VALID_STANDARD_SYMBOLS,
        &[],
        PRELUDE_MODULE,
        PRELUDE_BUILTIN_MODULE,
        BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        vec![valid_adt_descriptor()],
    ))
    .expect("all providers validate");

    let application_adts =
        AdtRegistry::from_module_with_base(&empty_module(), &registries.builtin_adts);
    let boxed = crate::semantic_model::Type::named("Boxed", Vec::new());

    assert!(
        application_adts.descriptor_for_type(&boxed).is_some(),
        "application ADT lookup should consume the published source-less ADT state"
    );
    assert!(matches!(
        application_adts.constructor(&path("Boxed", "Boxed"), None, &[]),
        crate::adt::ConstructorLookup::Found(_)
    ));
}

#[test]
fn invalid_type_syntax_descriptor_blocks_every_lookup_publication() {
    let result = build_source_less_lookup_registries(provider_set(
        VALID_STANDARD_SYMBOLS,
        &[],
        PRELUDE_MODULE,
        PRELUDE_BUILTIN_MODULE,
        INVALID_TYPE_SYNTAX,
        vec![valid_adt_descriptor()],
    ));
    let failure = result.expect_err("invalid type-syntax descriptor blocks publication");

    assert_eq!(failure.provider, "type_syntax");
    assert_eq!(failure.name, "result");
    assert_eq!(failure.name_class, SourceLessNameClass::Type);
    assert_eq!(failure.reason, InvalidStandardSymbolReason::InvalidCase);
}

#[test]
fn invalid_type_syntax_descriptor_blocks_production_type_parser_lookup() {
    let provider_set = SourceLessLookupProviderSet {
        qualified: VALID_STANDARD_SYMBOLS,
        compatibility_prelude: &[],
        self_hosting_prelude: &[],
        compiler_adapters: &[],
        standard_module: PRELUDE_MODULE,
        prelude_builtin_module: PRELUDE_BUILTIN_MODULE,
        builtin_type_syntax: INVALID_TYPE_SYNTAX,
        builtin_adts: vec![valid_adt_descriptor()],
    };

    with_source_less_lookup_provider_set_for_test(provider_set, || {
        let result = crate::type_syntax::type_annotation_reference_paths("Result<Int, String>");

        assert!(
            result
                .expect_err("invalid type-syntax descriptor blocks public type lookup")
                .contains("compiler-provided type `result` from `type_syntax` must start"),
            "public type lookup must preserve the source-less registry failure boundary"
        );
    });
}

#[test]
fn invalid_adt_descriptor_blocks_public_type_annotation_lookup() {
    let provider_set = SourceLessLookupProviderSet {
        qualified: VALID_STANDARD_SYMBOLS,
        compatibility_prelude: &[],
        self_hosting_prelude: &[],
        compiler_adapters: &[],
        standard_module: PRELUDE_MODULE,
        prelude_builtin_module: PRELUDE_BUILTIN_MODULE,
        builtin_type_syntax: BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        builtin_adts: vec![invalid_adt_descriptor()],
    };

    with_source_less_lookup_provider_set_for_test(provider_set, || {
        let result = crate::type_syntax::type_annotation_reference_paths("Result<Int, String>");

        assert!(
            result
                .expect_err("invalid ADT descriptor blocks public type lookup")
                .contains("compiler-provided constructor `boxed` from `adt` must start"),
            "public type lookup must not publish type-syntax state when ADT validation fails"
        );
    });
}

#[test]
fn duplicate_type_syntax_lookup_key_blocks_every_lookup_publication() {
    let result = build_source_less_lookup_registries(provider_set(
        VALID_STANDARD_SYMBOLS,
        &[],
        PRELUDE_MODULE,
        PRELUDE_BUILTIN_MODULE,
        DUPLICATE_TYPE_SYNTAX,
        vec![valid_adt_descriptor()],
    ));
    let failure = result.expect_err("duplicate type-syntax key blocks publication");
    let diagnostic = failure.diagnostic();

    assert_eq!(failure.code(), "toolchain.invalid_symbol_case");
    assert_eq!(failure.provider, "type_syntax");
    assert_eq!(failure.name, "Result");
    assert_eq!(failure.name_class, SourceLessNameClass::Type);
    assert_eq!(failure.required_initial(), "ascii_uppercase");
    assert_eq!(
        failure.reason,
        InvalidStandardSymbolReason::DuplicateLookupKey
    );
    assert_eq!(
        diagnostic.message,
        "compiler-provided type lookup key `Result` from `type_syntax` is duplicated"
    );
    assert_eq!(
        diagnostic.details.to_json(),
        "{\"provider\":\"type_syntax\",\"name\":\"Result\",\"name_class\":\"type\",\"required_initial\":\"ascii_uppercase\"}"
    );
}

#[test]
fn duplicate_type_syntax_lookup_key_blocks_production_type_parser_lookup() {
    let provider_set = SourceLessLookupProviderSet {
        qualified: VALID_STANDARD_SYMBOLS,
        compatibility_prelude: &[],
        self_hosting_prelude: &[],
        compiler_adapters: &[],
        standard_module: PRELUDE_MODULE,
        prelude_builtin_module: PRELUDE_BUILTIN_MODULE,
        builtin_type_syntax: DUPLICATE_TYPE_SYNTAX,
        builtin_adts: vec![valid_adt_descriptor()],
    };

    with_source_less_lookup_provider_set_for_test(provider_set, || {
        let result = crate::type_syntax::type_annotation_reference_paths("Result<Int, String>");

        assert!(
            result
                .expect_err("duplicate type-syntax descriptor blocks public type lookup")
                .contains(
                    "compiler-provided type lookup key `Result` from `type_syntax` is duplicated"
                ),
            "public type lookup must preserve duplicate type-syntax publication failure"
        );
    });
}

#[test]
fn invalid_standard_module_key_blocks_registry_publication() {
    let result = build_source_less_lookup_registries(provider_set(
        VALID_STANDARD_SYMBOLS,
        &[],
        "Prelude",
        PRELUDE_BUILTIN_MODULE,
        BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        vec![valid_adt_descriptor()],
    ));
    let failure = result.expect_err("invalid standard module key blocks publication");

    assert_eq!(failure.provider, "standard_names");
    assert_eq!(failure.name, "Prelude");
    assert_eq!(failure.name_class, SourceLessNameClass::Module);
}

#[test]
fn invalid_prelude_builtin_module_key_blocks_lookup_publication() {
    let result = build_source_less_lookup_registries(provider_set(
        VALID_STANDARD_SYMBOLS,
        VALID_STANDARD_SYMBOLS,
        PRELUDE_MODULE,
        "PreludeBuiltin",
        BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        vec![valid_adt_descriptor()],
    ));
    let failure = result.expect_err("invalid prelude_builtin key blocks publication");

    assert_eq!(failure.provider, "compiler_adapter");
    assert_eq!(failure.name, "PreludeBuiltin");
    assert_eq!(failure.name_class, SourceLessNameClass::Module);
}

#[test]
fn invalid_prelude_builtin_module_key_blocks_publication_without_adapters() {
    let result = build_source_less_lookup_registries(provider_set(
        VALID_STANDARD_SYMBOLS,
        &[],
        PRELUDE_MODULE,
        "PreludeBuiltin",
        BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        vec![valid_adt_descriptor()],
    ));
    let failure = result.expect_err("invalid prelude_builtin key blocks publication");

    assert_eq!(failure.code(), "toolchain.invalid_symbol_case");
    assert_eq!(failure.provider, "compiler_adapter");
    assert_eq!(failure.name, "PreludeBuiltin");
    assert_eq!(failure.name_class, SourceLessNameClass::Module);
}

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
