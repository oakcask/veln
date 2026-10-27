use veln_ast::SurfaceModule;

pub(super) fn should_validate_standard_bodies(module: &SurfaceModule) -> bool {
    !module
        .functions
        .iter()
        .any(|function| function.package_name.as_deref() != Some(veln_stdlib::PACKAGE_NAME))
}
