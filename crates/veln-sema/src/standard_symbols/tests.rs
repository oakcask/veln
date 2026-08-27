use std::collections::BTreeSet;

use super::*;

fn path(module: &str, name: &str) -> Vec<String> {
    vec![module.to_string(), name.to_string()]
}

#[test]
fn descriptor_table_carries_runtime_effect_metadata() {
    let symbol = qualified_symbol(&path("stdio", "println")).expect("stdio descriptor");

    assert_eq!(symbol.kind, StandardSymbolKind::Runtime);
    assert_eq!(symbol.name_class, SourceLessNameClass::Function);
    assert_eq!(symbol.effects, ["stdio"]);
    assert_eq!(symbol.lowering, Some("runtime.stdio.println"));
    assert_eq!(
        symbol.stability,
        StandardSymbolStability::RequiredForSelfHosting
    );
    assert_eq!(effect_strings(symbol), vec!["stdio"]);
}

#[test]
fn descriptor_table_carries_prelude_purity_metadata() {
    let symbol = prelude_symbol("float_add").expect("prelude descriptor");

    assert_eq!(symbol.kind, StandardSymbolKind::Prelude);
    assert_eq!(symbol.name_class, SourceLessNameClass::Function);
    assert!(symbol.effects.is_empty());
    assert_eq!(symbol.lowering, None);
    assert_eq!(symbol.stability, StandardSymbolStability::CompatibilityOnly);
}

#[test]
fn compiler_adapter_descriptors_carry_pure_metadata() {
    for name in COMPILER_ADAPTER_NAMES.iter().copied() {
        let symbol = compiler_adapter_symbol(name).expect("compiler adapter descriptor");
        assert_eq!(symbol.kind, StandardSymbolKind::Prelude);
        assert_eq!(symbol.name_class, SourceLessNameClass::Function);
        assert!(symbol.effects.is_empty());
        assert_eq!(symbol.lowering, None);
        if private_compiler_adapter_name(name) {
            assert!(
                compiler_adapter_symbol(name).is_some(),
                "prelude_builtin descriptor {name} should stay source-resolvable"
            );
            assert_eq!(prelude_symbol(name), None);
        } else {
            assert_eq!(prelude_symbol(name), Some(symbol));
        }
    }
}

#[test]
fn source_lookup_registry_accepts_current_generated_tables() {
    let registry = standard_symbol_registry().expect("standard symbol registry");

    assert_eq!(registry.qualified.len(), QUALIFIED_SYMBOLS.len());
    assert!(
        registry
            .qualified
            .iter()
            .any(|symbol| { symbol.module == Some("stdio") && symbol.name == "println" })
    );
    assert!(
        registry
            .prelude
            .iter()
            .any(|symbol| symbol.name == "float_add")
    );
    assert!(registry.prelude.iter().any(|symbol| symbol.name == "byte"));
    assert!(
        !registry
            .prelude
            .iter()
            .any(|symbol| private_compiler_adapter_name(symbol.name))
    );
}

#[test]
fn invalid_source_lookup_module_segment_fails_atomically() {
    const INVALID_QUALIFIED: &[StandardSymbolDescriptor] = &[StandardSymbolDescriptor {
        module: Some("Std"),
        name: "print",
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Runtime,
        effects: PURE_EFFECTS,
        lowering: Some("runtime.Std.print"),
        signature: None,
        stability: StandardSymbolStability::RequiredForSelfHosting,
    }];

    let failure =
        build_standard_symbol_registry(INVALID_QUALIFIED, &[], &[], &[]).expect_err("case failure");

    assert_eq!(failure.code(), "toolchain.invalid_symbol_case");
    assert_eq!(failure.provider, "runtime");
    assert_eq!(failure.name, "Std");
    assert_eq!(failure.name_class, SourceLessNameClass::Module);
    assert_eq!(failure.required_initial(), "ascii_lowercase");
}

#[test]
fn invalid_source_lookup_symbol_name_reports_descriptor_details() {
    const INVALID_ADAPTER: &[StandardSymbolDescriptor] = &[StandardSymbolDescriptor {
        module: None,
        name: "Byte",
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Prelude,
        effects: PURE_EFFECTS,
        lowering: None,
        signature: None,
        stability: StandardSymbolStability::CompatibilityOnly,
    }];

    let failure =
        build_standard_symbol_registry(&[], &[], &[], INVALID_ADAPTER).expect_err("case failure");
    let diagnostic = failure.diagnostic();

    assert_eq!(failure.provider, "compiler_adapter");
    assert_eq!(failure.name, "Byte");
    assert_eq!(failure.name_class, SourceLessNameClass::Function);
    assert_eq!(diagnostic.id, "toolchain.invalid_symbol_case");
    assert!(diagnostic.span.is_none());
    assert_eq!(
        diagnostic.details.to_json(),
        "{\"provider\":\"compiler_adapter\",\"name\":\"Byte\",\"name_class\":\"function\",\"required_initial\":\"ascii_lowercase\"}"
    );
}

#[test]
fn invalid_descriptor_prevents_partial_lookup_registry() {
    const VALID_QUALIFIED: &[StandardSymbolDescriptor] = &[StandardSymbolDescriptor {
        module: Some("stdio"),
        name: "print",
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Runtime,
        effects: PURE_EFFECTS,
        lowering: Some("runtime.stdio.print"),
        signature: None,
        stability: StandardSymbolStability::RequiredForSelfHosting,
    }];
    const INVALID_PRELUDE: &[StandardSymbolDescriptor] = &[StandardSymbolDescriptor {
        module: None,
        name: "Float_add",
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Prelude,
        effects: PURE_EFFECTS,
        lowering: None,
        signature: None,
        stability: StandardSymbolStability::CompatibilityOnly,
    }];

    let result = build_standard_symbol_registry(VALID_QUALIFIED, INVALID_PRELUDE, &[], &[]);

    assert!(result.is_err());
}

#[test]
fn checked_lookup_reports_invalid_registry_instead_of_lookup_miss() {
    let invalid_registry = Err(InvalidStandardSymbolCase {
        provider: "prelude",
        name: "Float_add".to_string(),
        name_class: SourceLessNameClass::Function,
    });
    let lookup = checked_prelude_symbol_in_registry(invalid_registry, "missing_name");
    let failure = lookup.expect_err("invalid registry blocks lookup");

    assert_eq!(failure.code(), "toolchain.invalid_symbol_case");
    assert_eq!(failure.provider, "prelude");
    assert_eq!(failure.name, "Float_add");
    assert_eq!(failure.name_class, SourceLessNameClass::Function);
    assert_eq!(failure.required_initial(), "ascii_lowercase");
}

#[test]
fn checked_qualified_lookup_reports_invalid_registry_instead_of_lookup_miss() {
    let invalid_registry = Err(InvalidStandardSymbolCase {
        provider: "runtime",
        name: "Std".to_string(),
        name_class: SourceLessNameClass::Module,
    });
    let lookup = checked_qualified_symbol_in_registry(invalid_registry, &path("stdio", "println"));
    let failure = lookup.expect_err("invalid registry blocks qualified lookup");

    assert_eq!(failure.code(), "toolchain.invalid_symbol_case");
    assert_eq!(failure.provider, "runtime");
    assert_eq!(failure.name, "Std");
    assert_eq!(failure.name_class, SourceLessNameClass::Module);
    assert_eq!(failure.required_initial(), "ascii_lowercase");
}

#[test]
fn duplicate_qualified_lookup_key_fails_atomically() {
    const DUPLICATE_QUALIFIED: &[StandardSymbolDescriptor] = &[
        StandardSymbolDescriptor {
            module: Some("stdio"),
            name: "print",
            name_class: SourceLessNameClass::Function,
            kind: StandardSymbolKind::Runtime,
            effects: PURE_EFFECTS,
            lowering: Some("runtime.stdio.print"),
            signature: None,
            stability: StandardSymbolStability::RequiredForSelfHosting,
        },
        StandardSymbolDescriptor {
            module: Some("stdio"),
            name: "print",
            name_class: SourceLessNameClass::Function,
            kind: StandardSymbolKind::Runtime,
            effects: PURE_EFFECTS,
            lowering: Some("runtime.stdio.print_duplicate"),
            signature: None,
            stability: StandardSymbolStability::RequiredForSelfHosting,
        },
    ];

    let failure = build_standard_symbol_registry(DUPLICATE_QUALIFIED, &[], &[], &[])
        .expect_err("duplicate qualified lookup key");

    assert_eq!(failure.code(), "toolchain.invalid_symbol_case");
    assert_eq!(failure.provider, "runtime");
    assert_eq!(failure.name, "stdio::print");
    assert_eq!(failure.name_class, SourceLessNameClass::Function);
    assert_eq!(failure.required_initial(), "ascii_lowercase");
}

#[test]
fn duplicate_prelude_lookup_key_fails_atomically() {
    const DUPLICATE_PRELUDE: &[StandardSymbolDescriptor] = &[
        StandardSymbolDescriptor {
            module: None,
            name: "float_add",
            name_class: SourceLessNameClass::Function,
            kind: StandardSymbolKind::Prelude,
            effects: PURE_EFFECTS,
            lowering: None,
            signature: None,
            stability: StandardSymbolStability::CompatibilityOnly,
        },
        StandardSymbolDescriptor {
            module: None,
            name: "float_add",
            name_class: SourceLessNameClass::Function,
            kind: StandardSymbolKind::Prelude,
            effects: PURE_EFFECTS,
            lowering: None,
            signature: None,
            stability: StandardSymbolStability::CompatibilityOnly,
        },
    ];

    let failure = build_standard_symbol_registry(&[], DUPLICATE_PRELUDE, &[], &[])
        .expect_err("duplicate prelude lookup key");

    assert_eq!(failure.code(), "toolchain.invalid_symbol_case");
    assert_eq!(failure.provider, "prelude");
    assert_eq!(failure.name, "float_add");
    assert_eq!(failure.name_class, SourceLessNameClass::Function);
    assert_eq!(failure.required_initial(), "ascii_lowercase");
}

#[test]
fn prelude_builtin_compiler_adapter_names_are_validated_but_not_bare_prelude() {
    const PRIVATE_ADAPTER_WITH_INVALID_MODULE: &[StandardSymbolDescriptor] =
        &[StandardSymbolDescriptor {
            module: Some("Internal"),
            name: "byte_decode_http2_frame",
            name_class: SourceLessNameClass::Function,
            kind: StandardSymbolKind::Prelude,
            effects: PURE_EFFECTS,
            lowering: None,
            signature: None,
            stability: StandardSymbolStability::CompatibilityOnly,
        }];

    let failure =
        build_standard_symbol_registry(&[], &[], &[], PRIVATE_ADAPTER_WITH_INVALID_MODULE)
            .expect_err("prelude_builtin adapter participates in source lookup");

    assert_eq!(failure.code(), "toolchain.invalid_symbol_case");
    assert_eq!(failure.provider, "compiler_adapter");
    assert_eq!(failure.name, "Internal");
    assert_eq!(failure.name_class, SourceLessNameClass::Module);
    assert_eq!(failure.required_initial(), "ascii_lowercase");
}

#[test]
fn compatibility_prelude_helpers_carry_only_intrinsic_metadata() {
    for symbol in compatibility_prelude_symbols() {
        assert_eq!(symbol.kind, StandardSymbolKind::Prelude);
        assert_eq!(symbol.lowering, None);
        assert!(symbol.effects.is_empty());
    }
}

#[test]
fn vec_fold_is_declared_by_the_standard_package() {
    let symbol = prelude_symbol("vec_fold").expect("vec_fold descriptor");
    let source = veln_stdlib::package_bundle()
        .files
        .iter()
        .find(|file| file.path == "prelude.veln")
        .expect("prelude source");
    assert_eq!(symbol.kind, StandardSymbolKind::Prelude);
    assert!(source.text.contains("fn vec_fold("));
}

#[test]
fn standard_package_private_helpers_are_not_compiler_adapters() {
    for name in STANDARD_PACKAGE_PRIVATE_HELPERS {
        assert_eq!(prelude_symbol(name), None);
    }
}

#[test]
fn deferred_dictionary_traversal_helpers_are_not_prelude_descriptors() {
    for name in ["dict_keys", "dict_values"] {
        assert_eq!(prelude_symbol(name), None, "{name}");
    }
}

#[test]
fn no_deferred_pure_helpers_remain_outside_the_standard_package() {
    assert_eq!(SELF_HOSTING_CANDIDATE_PRELUDE_SYMBOLS.iter().next(), None);
}

#[test]
fn compiler_adapter_boundary_matches_current_prelude_split() {
    let compiler_adapters = COMPILER_ADAPTER_SYMBOLS
        .iter()
        .map(|symbol| symbol.name)
        .collect::<Vec<_>>();
    let compatibility_intrinsics = compatibility_prelude_symbols()
        .map(|symbol| symbol.name)
        .collect::<Vec<_>>();

    assert_eq!(compiler_adapters, COMPILER_ADAPTER_NAMES);
    assert_eq!(
        compatibility_intrinsics,
        [
            "float_negate",
            "float_add",
            "float_subtract",
            "float_multiply",
            "float_divide",
            "float_less",
            "float_less_equal",
            "float_greater",
            "float_greater_equal",
        ]
    );
}

#[test]
fn public_compiler_adapter_names_are_public_prelude_functions() {
    let source = veln_stdlib::package_bundle()
        .files
        .iter()
        .find(|file| file.path == "prelude.veln")
        .expect("prelude source");
    let file = veln_source::SourceFile::new(source.path, source.text);
    let parsed = veln_syntax::parse(&file);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = veln_ast::lower_surface_ast(&parsed.tree);
    let public_names = module
        .functions
        .iter()
        .filter(|function| function.visibility == veln_ast::Visibility::Public)
        .filter_map(|function| function.name.as_deref())
        .collect::<BTreeSet<_>>();

    for name in COMPILER_ADAPTER_NAMES
        .iter()
        .copied()
        .filter(|name| prelude_symbol(name).is_some())
    {
        assert!(
            public_names.contains(name),
            "missing public std function {name}"
        );
    }
}

#[test]
fn qualified_descriptors_have_unique_source_names() {
    let mut names = BTreeSet::new();

    for symbol in QUALIFIED_SYMBOLS {
        let module = symbol.module.expect("qualified symbol has a module");
        assert!(
            names.insert((module, symbol.name)),
            "duplicate qualified symbol {module}::{}",
            symbol.name
        );
    }
}

#[test]
fn prelude_descriptors_have_unique_source_names() {
    let mut names = BTreeSet::new();

    for symbol in prelude_symbols() {
        assert_eq!(symbol.module, None);
        assert!(
            names.insert(symbol.name),
            "duplicate prelude symbol {}",
            symbol.name
        );
    }
}
