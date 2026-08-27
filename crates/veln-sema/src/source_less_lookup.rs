use std::sync::OnceLock;

use crate::adt::{AdtDescriptor, build_builtin_descriptors, validate_adt_lookup_descriptors};
use crate::standard_symbols::{
    COMPILER_ADAPTER_SYMBOLS, FLOAT_COMPATIBILITY_PRELUDE_SYMBOLS, InvalidStandardSymbolCase,
    QUALIFIED_SYMBOLS, SELF_HOSTING_CANDIDATE_PRELUDE_SYMBOLS, StandardSymbolDescriptor,
    StandardSymbolRegistry, build_standard_symbol_registry,
};

#[derive(Debug)]
pub(crate) struct SourceLessLookupRegistries {
    standard_symbols: StandardSymbolRegistry,
    builtin_adts: Vec<AdtDescriptor>,
}

pub(crate) fn validate_source_less_lookup_registries() -> Result<(), InvalidStandardSymbolCase> {
    source_less_lookup_registries().map(|_| ())
}

pub(crate) fn standard_symbol_registry()
-> Result<&'static StandardSymbolRegistry, InvalidStandardSymbolCase> {
    Ok(&source_less_lookup_registries()?.standard_symbols)
}

pub(crate) fn builtin_adt_descriptors()
-> Result<&'static [AdtDescriptor], InvalidStandardSymbolCase> {
    Ok(&source_less_lookup_registries()?.builtin_adts)
}

pub(crate) fn qualified_symbol_checked(
    segments: &[String],
) -> Result<Option<&'static StandardSymbolDescriptor>, InvalidStandardSymbolCase> {
    Ok(standard_symbol_registry()?.qualified_symbol(segments))
}

pub(crate) fn prelude_symbol_checked(
    name: &str,
) -> Result<Option<&'static StandardSymbolDescriptor>, InvalidStandardSymbolCase> {
    Ok(standard_symbol_registry()?.prelude_symbol(name))
}

pub(crate) fn compiler_adapter_symbol_checked(
    name: &str,
) -> Result<Option<&'static StandardSymbolDescriptor>, InvalidStandardSymbolCase> {
    Ok(standard_symbol_registry()?.compiler_adapter_symbol(name))
}

fn source_less_lookup_registries()
-> Result<&'static SourceLessLookupRegistries, InvalidStandardSymbolCase> {
    static REGISTRIES: OnceLock<Result<SourceLessLookupRegistries, InvalidStandardSymbolCase>> =
        OnceLock::new();
    REGISTRIES
        .get_or_init(|| {
            build_source_less_lookup_registries(
                QUALIFIED_SYMBOLS,
                FLOAT_COMPATIBILITY_PRELUDE_SYMBOLS,
                SELF_HOSTING_CANDIDATE_PRELUDE_SYMBOLS,
                COMPILER_ADAPTER_SYMBOLS,
                build_builtin_descriptors(),
            )
        })
        .as_ref()
        .map_err(Clone::clone)
}

pub(crate) fn build_source_less_lookup_registries(
    qualified: &'static [StandardSymbolDescriptor],
    compatibility_prelude: &'static [StandardSymbolDescriptor],
    self_hosting_prelude: &'static [StandardSymbolDescriptor],
    compiler_adapters: &'static [StandardSymbolDescriptor],
    builtin_adts: Vec<AdtDescriptor>,
) -> Result<SourceLessLookupRegistries, InvalidStandardSymbolCase> {
    let standard_symbols = build_standard_symbol_registry(
        qualified,
        compatibility_prelude,
        self_hosting_prelude,
        compiler_adapters,
    )?;
    validate_adt_lookup_descriptors("adt", &builtin_adts)?;
    Ok(SourceLessLookupRegistries {
        standard_symbols,
        builtin_adts,
    })
}

#[cfg(test)]
mod tests {
    use veln_ast::Visibility;

    use super::*;
    use crate::adt::{AdtDescriptor, AdtVariantDescriptor, AdtVariantKind};
    use crate::standard_symbols::{
        InvalidStandardSymbolReason, SourceLessNameClass, StandardSymbolKind,
        StandardSymbolStability,
    };

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

    fn valid_adt_descriptor() -> AdtDescriptor {
        AdtDescriptor {
            type_name: "Boxed".to_string(),
            module_name: None,
            type_parameters: Vec::new(),
            variants: vec![AdtVariantDescriptor {
                name: "Boxed".to_string(),
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

    #[test]
    fn invalid_adt_descriptor_blocks_standard_symbol_publication() {
        let result = build_source_less_lookup_registries(
            VALID_STANDARD_SYMBOLS,
            &[],
            &[],
            &[],
            vec![invalid_adt_descriptor()],
        );
        let failure = result.expect_err("invalid ADT blocks all source-less lookup");

        assert_eq!(failure.provider, "adt");
        assert_eq!(failure.name, "boxed");
        assert_eq!(failure.name_class, SourceLessNameClass::Constructor);
        assert_eq!(failure.reason, InvalidStandardSymbolReason::InvalidCase);
    }

    #[test]
    fn invalid_standard_symbol_descriptor_blocks_adt_publication() {
        let result = build_source_less_lookup_registries(
            INVALID_STANDARD_SYMBOLS,
            &[],
            &[],
            &[],
            vec![valid_adt_descriptor()],
        );
        let failure = result.expect_err("invalid standard symbol blocks all source-less lookup");

        assert_eq!(failure.provider, "runtime");
        assert_eq!(failure.name, "Std");
        assert_eq!(failure.name_class, SourceLessNameClass::Module);
        assert_eq!(failure.reason, InvalidStandardSymbolReason::InvalidCase);
    }

    #[test]
    fn shared_registry_exposes_lookup_only_after_all_providers_validate() {
        let registries = build_source_less_lookup_registries(
            VALID_STANDARD_SYMBOLS,
            &[],
            &[],
            &[],
            vec![valid_adt_descriptor()],
        )
        .expect("all providers validate");

        assert!(
            registries
                .standard_symbols
                .qualified_symbol(&path("stdio", "print"))
                .is_some()
        );
        assert_eq!(registries.builtin_adts[0].type_name, "Boxed");
    }
}
