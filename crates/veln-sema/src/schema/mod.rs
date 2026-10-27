pub(crate) mod dispatch;
pub(crate) mod ir;
pub(crate) mod primitives;
pub(crate) mod reserved_layout;

pub(crate) use dispatch::*;
pub(crate) use primitives::*;
pub(crate) use reserved_layout::*;

/// Preserve source names for unique schemas and disambiguate colliding runtime providers.
pub(crate) fn schema_link_name(
    module: &veln_ast::SurfaceModule,
    schema: &veln_ast::SchemaDecl,
) -> String {
    let name = schema.name.as_deref().unwrap_or("<missing>");
    let collision = module.schemas.iter().any(|other| {
        other.name.as_deref() == Some(name)
            && (other.package_name != schema.package_name
                || other.module_name != schema.module_name)
    });
    if !collision {
        return name.to_string();
    }
    match (
        schema.package_name.as_deref(),
        schema.module_name.as_deref(),
    ) {
        (Some(package), Some(owner))
            if owner == package || owner.starts_with(&format!("{package}::")) =>
        {
            format!("{owner}::{name}")
        }
        (Some(package), Some(owner)) => format!("{package}::{owner}::{name}"),
        (None, Some(owner)) => format!("workspace::{owner}::{name}"),
        (Some(package), None) => format!("{package}::{name}"),
        (None, None) => format!("workspace::{name}"),
    }
}
