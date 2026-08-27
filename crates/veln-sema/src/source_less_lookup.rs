#[cfg(test)]
use std::collections::BTreeSet;
use std::sync::OnceLock;

use crate::adt::{AdtDescriptor, AdtRegistry, build_builtin_descriptors};
use crate::source_less_names::{
    InvalidStandardSymbolCase, SourceLessNameClass, validate_source_less_name,
};
use crate::standard_names::PRELUDE_MODULE;
use crate::standard_symbols::{
    COMPILER_ADAPTER_SYMBOLS, FLOAT_COMPATIBILITY_PRELUDE_SYMBOLS, QUALIFIED_SYMBOLS,
    SELF_HOSTING_CANDIDATE_PRELUDE_SYMBOLS, StandardSymbolDescriptor, StandardSymbolRegistry,
    build_standard_symbol_registry_with_modules, private_compiler_adapter_name,
};
use crate::type_syntax::{
    BUILTIN_TYPE_SYNTAX_DESCRIPTORS, BuiltinTypeSyntaxDescriptor, BuiltinTypeSyntaxRegistry,
};

pub(crate) const PRELUDE_BUILTIN_MODULE: &str = "prelude_builtin";

#[derive(Debug)]
pub(crate) struct SourceLessLookupRegistries {
    standard_module: &'static str,
    prelude_builtin_module: &'static str,
    standard_symbols: StandardSymbolRegistry,
    builtin_type_syntax: BuiltinTypeSyntaxRegistry,
    builtin_adts: AdtRegistry,
}

#[cfg_attr(test, derive(Clone))]
pub(crate) struct SourceLessLookupProviderSet {
    pub(crate) qualified: &'static [StandardSymbolDescriptor],
    pub(crate) compatibility_prelude: &'static [StandardSymbolDescriptor],
    pub(crate) self_hosting_prelude: &'static [StandardSymbolDescriptor],
    pub(crate) compiler_adapters: &'static [StandardSymbolDescriptor],
    pub(crate) standard_module: &'static str,
    pub(crate) prelude_builtin_module: &'static str,
    pub(crate) builtin_type_syntax: &'static [BuiltinTypeSyntaxDescriptor],
    pub(crate) builtin_adts: Vec<AdtDescriptor>,
}

pub(crate) fn validate_source_less_lookup_registries() -> Result<(), InvalidStandardSymbolCase> {
    with_source_less_lookup_registries(|registries| {
        let _ = registries.standard_module;
        let _ = registries.prelude_builtin_module;
        let _ = registries.standard_symbols.prelude_symbol("");
        let _ = registries.builtin_type_syntax.descriptors().len();
        let _ = registries.builtin_adts.descriptors().len();
    })
}

pub(crate) fn with_standard_symbol_registry<R>(
    lookup: impl FnOnce(&StandardSymbolRegistry) -> R,
) -> Result<R, InvalidStandardSymbolCase> {
    with_source_less_lookup_registries(|registries| lookup(&registries.standard_symbols))
}

pub(crate) fn qualified_symbol(segments: &[String]) -> Option<&'static StandardSymbolDescriptor> {
    with_standard_symbol_registry(|registry| registry.qualified_symbol(segments))
        .expect("source-less lookup registries are valid")
}

pub(crate) fn prelude_symbol(name: &str) -> Option<&'static StandardSymbolDescriptor> {
    if private_compiler_adapter_name(name) {
        return None;
    }
    with_standard_symbol_registry(|registry| registry.prelude_symbol(name))
        .expect("source-less lookup registries are valid")
}

pub(crate) fn compiler_adapter_symbol(name: &str) -> Option<&'static StandardSymbolDescriptor> {
    with_standard_symbol_registry(|registry| registry.compiler_adapter_symbol(name))
        .expect("source-less lookup registries are valid")
}

pub(crate) fn prelude_builtin_module() -> &'static str {
    with_source_less_lookup_registries(|registries| registries.prelude_builtin_module)
        .expect("source-less lookup registries are valid")
}

pub(crate) fn standard_module() -> &'static str {
    with_source_less_lookup_registries(|registries| registries.standard_module)
        .expect("source-less lookup registries are valid")
}

pub(crate) fn is_reserved_source_less_module(module: &str) -> bool {
    module == standard_module() || module == prelude_builtin_module()
}

pub(crate) fn with_builtin_adt_registry<R>(
    lookup: impl FnOnce(&AdtRegistry) -> R,
) -> Result<R, InvalidStandardSymbolCase> {
    with_source_less_lookup_registries(|registries| lookup(&registries.builtin_adts))
}

pub(crate) fn with_builtin_type_syntax_registry<R>(
    lookup: impl FnOnce(&BuiltinTypeSyntaxRegistry) -> R,
) -> Result<R, InvalidStandardSymbolCase> {
    with_source_less_lookup_registries(|registries| lookup(&registries.builtin_type_syntax))
}

pub(crate) fn published_builtin_adt_registry() -> Result<AdtRegistry, InvalidStandardSymbolCase> {
    with_builtin_adt_registry(Clone::clone)
}

fn with_source_less_lookup_registries<R>(
    lookup: impl FnOnce(&SourceLessLookupRegistries) -> R,
) -> Result<R, InvalidStandardSymbolCase> {
    #[cfg(test)]
    {
        let mut lookup = Some(lookup);
        if let Some(result) = with_test_provider_registries(|registries| {
            lookup
                .take()
                .expect("source-less lookup closure is called once")(registries)
        }) {
            return result;
        }
        let lookup = lookup.expect("source-less lookup closure has not been called");
        source_less_lookup_registries().map(lookup)
    }

    #[cfg(not(test))]
    {
        source_less_lookup_registries().map(lookup)
    }
}

fn source_less_lookup_registries()
-> Result<&'static SourceLessLookupRegistries, InvalidStandardSymbolCase> {
    static REGISTRIES: OnceLock<Result<SourceLessLookupRegistries, InvalidStandardSymbolCase>> =
        OnceLock::new();
    REGISTRIES
        .get_or_init(|| {
            build_source_less_lookup_registries(SourceLessLookupProviderSet {
                qualified: QUALIFIED_SYMBOLS,
                compatibility_prelude: FLOAT_COMPATIBILITY_PRELUDE_SYMBOLS,
                self_hosting_prelude: SELF_HOSTING_CANDIDATE_PRELUDE_SYMBOLS,
                compiler_adapters: COMPILER_ADAPTER_SYMBOLS,
                standard_module: PRELUDE_MODULE,
                prelude_builtin_module: PRELUDE_BUILTIN_MODULE,
                builtin_type_syntax: BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
                builtin_adts: build_builtin_descriptors(),
            })
        })
        .as_ref()
        .map_err(Clone::clone)
}

pub(crate) fn build_source_less_lookup_registries(
    provider_set: SourceLessLookupProviderSet,
) -> Result<SourceLessLookupRegistries, InvalidStandardSymbolCase> {
    validate_source_less_name(
        "compiler_adapter",
        provider_set.prelude_builtin_module,
        SourceLessNameClass::Module,
    )?;
    let standard_symbols = build_standard_symbol_registry_with_modules(
        provider_set.prelude_builtin_module,
        provider_set.qualified,
        provider_set.compatibility_prelude,
        provider_set.self_hosting_prelude,
        provider_set.compiler_adapters,
    )?;
    validate_source_less_name(
        "standard_names",
        provider_set.standard_module,
        SourceLessNameClass::Module,
    )?;
    let builtin_type_syntax = BuiltinTypeSyntaxRegistry::from_validated_source_less_descriptors(
        provider_set.builtin_type_syntax,
    )?;
    let builtin_adts =
        AdtRegistry::from_validated_source_less_descriptors(provider_set.builtin_adts)?;
    Ok(SourceLessLookupRegistries {
        standard_module: provider_set.standard_module,
        prelude_builtin_module: provider_set.prelude_builtin_module,
        standard_symbols,
        builtin_type_syntax,
        builtin_adts,
    })
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SourceLessLookupRoute {
    pub(crate) provider: &'static str,
    pub(crate) lookup_key: String,
    pub(crate) name_class: SourceLessNameClass,
}

#[cfg(test)]
pub(crate) fn production_source_less_lookup_routes_for_test()
-> Result<Vec<SourceLessLookupRoute>, InvalidStandardSymbolCase> {
    let registries = build_source_less_lookup_registries(SourceLessLookupProviderSet {
        qualified: QUALIFIED_SYMBOLS,
        compatibility_prelude: FLOAT_COMPATIBILITY_PRELUDE_SYMBOLS,
        self_hosting_prelude: SELF_HOSTING_CANDIDATE_PRELUDE_SYMBOLS,
        compiler_adapters: COMPILER_ADAPTER_SYMBOLS,
        standard_module: PRELUDE_MODULE,
        prelude_builtin_module: PRELUDE_BUILTIN_MODULE,
        builtin_type_syntax: BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
        builtin_adts: build_builtin_descriptors(),
    })?;
    let mut routes = BTreeSet::new();

    collect_standard_symbol_routes(&registries, &mut routes);
    collect_builtin_type_syntax_routes(&registries, &mut routes);
    collect_builtin_adt_routes(&registries, &mut routes);

    Ok(routes.into_iter().collect())
}

#[cfg(test)]
fn collect_standard_symbol_routes(
    registries: &SourceLessLookupRegistries,
    routes: &mut BTreeSet<SourceLessLookupRoute>,
) {
    for symbol in registries.standard_symbols.qualified_symbols() {
        if let Some(module) = symbol.module {
            routes.insert(SourceLessLookupRoute {
                provider: "runtime",
                lookup_key: format!("{module}::{}", symbol.name),
                name_class: symbol.name_class,
            });
        }
    }
    for symbol in registries.standard_symbols.prelude_symbols() {
        routes.insert(SourceLessLookupRoute {
            provider: "prelude",
            lookup_key: symbol.name.to_string(),
            name_class: symbol.name_class,
        });
    }
    for symbol in registries.standard_symbols.compiler_adapter_symbols() {
        routes.insert(SourceLessLookupRoute {
            provider: "compiler_adapter",
            lookup_key: format!("prelude_builtin::{}", symbol.name),
            name_class: symbol.name_class,
        });
    }
    routes.insert(SourceLessLookupRoute {
        provider: "standard_names",
        lookup_key: registries.standard_module.to_string(),
        name_class: SourceLessNameClass::Module,
    });
}

#[cfg(test)]
fn collect_builtin_type_syntax_routes(
    registries: &SourceLessLookupRegistries,
    routes: &mut BTreeSet<SourceLessLookupRoute>,
) {
    for descriptor in registries.builtin_type_syntax.descriptors() {
        routes.insert(SourceLessLookupRoute {
            provider: "type_syntax",
            lookup_key: descriptor.name.to_string(),
            name_class: descriptor.name_class,
        });
    }
}

#[cfg(test)]
fn collect_builtin_adt_routes(
    registries: &SourceLessLookupRegistries,
    routes: &mut BTreeSet<SourceLessLookupRoute>,
) {
    for descriptor in registries.builtin_adts.descriptors() {
        routes.insert(SourceLessLookupRoute {
            provider: "adt",
            lookup_key: adt_type_route_key(descriptor),
            name_class: descriptor.name_class,
        });
        for variant in &descriptor.variants {
            routes.insert(SourceLessLookupRoute {
                provider: "adt",
                lookup_key: adt_constructor_route_key(descriptor, variant.name.as_str()),
                name_class: variant.name_class,
            });
        }
    }
}

#[cfg(test)]
fn adt_type_route_key(descriptor: &AdtDescriptor) -> String {
    match descriptor.module_name.as_deref() {
        Some(module_name) => format!("{module_name}::{}", descriptor.type_name),
        None => descriptor.type_name.clone(),
    }
}

#[cfg(test)]
fn adt_constructor_route_key(descriptor: &AdtDescriptor, variant_name: &str) -> String {
    match descriptor.module_name.as_deref() {
        Some(module_name) => {
            format!("{module_name}::{}::{variant_name}", descriptor.type_name)
        }
        None => format!("{}::{variant_name}", descriptor.type_name),
    }
}

#[cfg(test)]
thread_local! {
    static TEST_PROVIDER_SET: std::cell::RefCell<Option<SourceLessLookupProviderSet>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(crate) fn with_source_less_lookup_provider_set_for_test<R>(
    provider_set: SourceLessLookupProviderSet,
    test: impl FnOnce() -> R,
) -> R {
    use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

    TEST_PROVIDER_SET.with(|current| {
        let previous = current.replace(Some(provider_set));
        let result = catch_unwind(AssertUnwindSafe(test));
        current.replace(previous);
        match result {
            Ok(result) => result,
            Err(payload) => resume_unwind(payload),
        }
    })
}

#[cfg(test)]
fn with_test_provider_registries<R>(
    lookup: impl FnOnce(&SourceLessLookupRegistries) -> R,
) -> Option<Result<R, InvalidStandardSymbolCase>> {
    TEST_PROVIDER_SET.with(|current| {
        current.borrow().clone().map(|provider_set| {
            let registries = build_source_less_lookup_registries(provider_set)?;
            Ok(lookup(&registries))
        })
    })
}

#[cfg(test)]
#[path = "source_less_lookup/tests.rs"]
mod tests;
