use super::*;

pub(super) fn path(module: &str, name: &str) -> Vec<String> {
    vec![module.to_string(), name.to_string()]
}

pub(super) const VALID_STANDARD_SYMBOLS: &[StandardSymbolDescriptor] =
    &[StandardSymbolDescriptor {
        module: Some("stdio"),
        name: "print",
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Runtime,
        effects: &[],
        lowering: Some("runtime.stdio.print"),
        signature: None,
        stability: StandardSymbolStability::RequiredForSelfHosting,
    }];

pub(super) const INVALID_STANDARD_SYMBOLS: &[StandardSymbolDescriptor] =
    &[StandardSymbolDescriptor {
        module: Some("Std"),
        name: "print",
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Runtime,
        effects: &[],
        lowering: Some("runtime.Std.print"),
        signature: None,
        stability: StandardSymbolStability::RequiredForSelfHosting,
    }];

pub(super) const UNCONSUMABLE_STANDARD_SYMBOL_KEY: &[StandardSymbolDescriptor] =
    &[StandardSymbolDescriptor {
        module: Some("foo::bar"),
        name: "print",
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Runtime,
        effects: &[],
        lowering: Some("runtime.foo.bar.print"),
        signature: None,
        stability: StandardSymbolStability::RequiredForSelfHosting,
    }];

pub(super) const RUNTIME_SYMBOL_WITH_QUALIFIED_LEAF: &[StandardSymbolDescriptor] =
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

pub(super) const RUNTIME_SYMBOL_WITH_NON_IDENTIFIER_LEAF: &[StandardSymbolDescriptor] =
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

pub(super) const PRELUDE_SYMBOL_WITH_QUALIFIED_LEAF: &[StandardSymbolDescriptor] =
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

pub(super) const PRELUDE_SYMBOL_WITH_CONTEXTUAL_LITERAL_LEAF: &[StandardSymbolDescriptor] =
    &[StandardSymbolDescriptor {
        module: None,
        name: "true",
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Prelude,
        effects: &[],
        lowering: None,
        signature: None,
        stability: StandardSymbolStability::CompatibilityOnly,
    }];

pub(super) const PUBLIC_COMPILER_ADAPTER_WITH_CONTEXTUAL_LITERAL_LEAF:
    &[StandardSymbolDescriptor] = &[StandardSymbolDescriptor {
    module: None,
    name: "false",
    name_class: SourceLessNameClass::Function,
    kind: StandardSymbolKind::Prelude,
    effects: &[],
    lowering: None,
    signature: None,
    stability: StandardSymbolStability::CompatibilityOnly,
}];

pub(super) const COMPILER_ADAPTER_WITH_NON_IDENTIFIER_LEAF: &[StandardSymbolDescriptor] =
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

pub(super) const INVALID_TYPE_SYNTAX: &[BuiltinTypeSyntaxDescriptor] =
    &[BuiltinTypeSyntaxDescriptor {
        name: "result",
        name_class: SourceLessNameClass::Type,
        arity: 2,
    }];
pub(super) const TYPE_SYNTAX_WITH_QUALIFIED_LEAF: &[BuiltinTypeSyntaxDescriptor] =
    &[BuiltinTypeSyntaxDescriptor {
        name: "Some::Inner",
        name_class: SourceLessNameClass::Type,
        arity: 0,
    }];
pub(super) const TYPE_SYNTAX_WITH_NON_IDENTIFIER_LEAF: &[BuiltinTypeSyntaxDescriptor] =
    &[BuiltinTypeSyntaxDescriptor {
        name: "Some-Inner",
        name_class: SourceLessNameClass::Type,
        arity: 0,
    }];
pub(super) const DUPLICATE_TYPE_SYNTAX: &[BuiltinTypeSyntaxDescriptor] = &[
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

pub(super) fn valid_adt_descriptor() -> AdtDescriptor {
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

pub(super) fn invalid_adt_descriptor() -> AdtDescriptor {
    let mut descriptor = valid_adt_descriptor();
    descriptor.variants[0].name = "boxed".to_string();
    descriptor
}

pub(super) fn adt_with_type_name(name: &str) -> AdtDescriptor {
    let mut descriptor = valid_adt_descriptor();
    descriptor.type_name = name.to_string();
    descriptor
}

pub(super) fn adt_with_constructor_name(name: &str) -> AdtDescriptor {
    let mut descriptor = valid_adt_descriptor();
    descriptor.variants[0].name = name.to_string();
    descriptor
}

pub(super) fn empty_module() -> SurfaceModule {
    SurfaceModule {
        module: None,
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: Vec::new(),
        schemas: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    }
}

pub(super) fn provider_set(
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

pub(super) fn assert_invalid_lookup_key(
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
