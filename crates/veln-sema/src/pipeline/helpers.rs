use veln_ast::SurfaceModule;

pub(super) fn should_validate_standard_bodies(module: &SurfaceModule) -> bool {
    !module.functions.iter().any(|function| {
        !function
            .module_name
            .as_deref()
            .is_some_and(|module| module.starts_with("std::"))
    })
}
