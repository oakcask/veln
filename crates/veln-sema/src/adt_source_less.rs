use veln_ast::SurfaceModule;

use crate::adt::registry::AdtRegistry;

#[cfg(test)]
thread_local! {
    static ADT_REGISTRY_FROM_MODULE_BUILDS: std::cell::Cell<usize> = const {
        std::cell::Cell::new(0)
    };
}

#[cfg(test)]
pub(crate) fn reset_adt_registry_from_module_builds() {
    ADT_REGISTRY_FROM_MODULE_BUILDS.set(0);
}

#[cfg(test)]
pub(crate) fn adt_registry_from_module_builds() -> usize {
    ADT_REGISTRY_FROM_MODULE_BUILDS.get()
}

impl AdtRegistry {
    pub(crate) fn from_module(module: &SurfaceModule) -> Self {
        #[cfg(test)]
        ADT_REGISTRY_FROM_MODULE_BUILDS.set(ADT_REGISTRY_FROM_MODULE_BUILDS.get() + 1);
        let builtin_adts = crate::source_less_lookup::published_builtin_adt_registry()
            .expect("source-less lookup registries are valid");
        Self::from_module_with_base(module, &builtin_adts)
    }
}
