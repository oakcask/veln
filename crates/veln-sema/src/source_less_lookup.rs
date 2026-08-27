use std::sync::OnceLock;

use crate::adt::{AdtDescriptor, AdtRegistry, build_builtin_descriptors};
use crate::source_less_names::InvalidStandardSymbolCase;
use crate::standard_symbols::{
    COMPILER_ADAPTER_SYMBOLS, FLOAT_COMPATIBILITY_PRELUDE_SYMBOLS, QUALIFIED_SYMBOLS,
    SELF_HOSTING_CANDIDATE_PRELUDE_SYMBOLS, StandardSymbolDescriptor, StandardSymbolRegistry,
    build_standard_symbol_registry,
};

#[derive(Debug)]
pub(crate) struct SourceLessLookupRegistries {
    standard_symbols: StandardSymbolRegistry,
    builtin_adts: AdtRegistry,
}

pub(crate) fn validate_source_less_lookup_registries() -> Result<(), InvalidStandardSymbolCase> {
    let registries = source_less_lookup_registries()?;
    let _ = registries.standard_symbols.prelude_symbol("");
    let _ = registries.builtin_adts.descriptors().len();
    Ok(())
}

#[cfg(test)]
pub(crate) fn standard_symbol_registry()
-> Result<&'static StandardSymbolRegistry, InvalidStandardSymbolCase> {
    Ok(&source_less_lookup_registries()?.standard_symbols)
}

#[cfg(test)]
pub(crate) fn builtin_adt_registry() -> Result<&'static AdtRegistry, InvalidStandardSymbolCase> {
    Ok(&source_less_lookup_registries()?.builtin_adts)
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
    let builtin_adts = AdtRegistry::from_validated_source_less_descriptors(builtin_adts)?;
    Ok(SourceLessLookupRegistries {
        standard_symbols,
        builtin_adts,
    })
}

#[cfg(test)]
mod tests {
    use veln_ast::{SurfaceModule, Visibility};

    use super::*;
    use crate::adt::{AdtDescriptor, AdtVariantDescriptor, AdtVariantKind};
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
    fn invalid_adt_descriptor_blocks_every_lookup_publication() {
        let result = build_source_less_lookup_registries(
            VALID_STANDARD_SYMBOLS,
            &[],
            &[],
            &[],
            vec![invalid_adt_descriptor()],
        );

        assert!(
            result.is_err(),
            "valid runtime symbols must not publish when ADT lookup validation fails"
        );
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
    fn invalid_standard_symbol_descriptor_blocks_every_lookup_publication() {
        let result = build_source_less_lookup_registries(
            INVALID_STANDARD_SYMBOLS,
            &[],
            &[],
            &[],
            vec![valid_adt_descriptor()],
        );

        assert!(
            result.is_err(),
            "valid built-in ADTs must not publish when standard symbol validation fails"
        );
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
        let option = crate::semantic_model::Type::named("Boxed", Vec::new());
        assert!(
            registries
                .builtin_adts
                .descriptor_for_type(&option)
                .is_some()
        );
    }

    #[test]
    fn published_adt_registry_seeds_application_lookup() {
        let registries = build_source_less_lookup_registries(
            VALID_STANDARD_SYMBOLS,
            &[],
            &[],
            &[],
            vec![valid_adt_descriptor()],
        )
        .expect("all providers validate");

        let application_adts =
            AdtRegistry::from_module_with_base(&empty_module(), Some(&registries.builtin_adts));
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
}
