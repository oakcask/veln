use std::collections::BTreeSet;

use super::*;

fn path(module: &str, name: &str) -> Vec<String> {
    vec![module.to_string(), name.to_string()]
}

#[test]
fn descriptor_table_carries_runtime_effect_metadata() {
    let symbol = qualified_symbol(&path("stdio", "println")).expect("stdio descriptor");

    assert_eq!(symbol.kind, StandardSymbolKind::Runtime);
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
    assert!(symbol.effects.is_empty());
    assert_eq!(symbol.lowering, None);
    assert_eq!(symbol.stability, StandardSymbolStability::CompatibilityOnly);
}

#[test]
fn compiler_adapter_descriptors_carry_pure_metadata() {
    for name in COMPILER_ADAPTER_NAMES.iter().copied() {
        let symbol = prelude_symbol(name).expect("prelude adapter descriptor");
        assert_eq!(symbol.kind, StandardSymbolKind::Prelude);
        assert!(symbol.effects.is_empty());
        assert_eq!(symbol.lowering, None);
    }
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
fn compiler_adapter_names_are_public_standard_package_functions() {
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

    for name in COMPILER_ADAPTER_NAMES {
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
