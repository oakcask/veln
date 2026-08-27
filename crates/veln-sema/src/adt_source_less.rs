use veln_ast::SurfaceModule;

use crate::adt::AdtRegistry;

impl AdtRegistry {
    pub(crate) fn from_module(module: &SurfaceModule) -> Self {
        let builtin_adts = crate::source_less_lookup::published_builtin_adt_registry()
            .expect("source-less lookup registries are valid");
        Self::from_module_with_base(module, &builtin_adts)
    }
}
