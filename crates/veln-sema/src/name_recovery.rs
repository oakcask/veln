use veln_ast::{NameClass, NameOccurrence, PublicAlias, SurfaceModule};

pub(crate) fn public_alias_has_invalid_target_leaf(
    module: &SurfaceModule,
    alias: &PublicAlias,
    class: Option<NameClass>,
) -> bool {
    module.invalid_names.iter().any(|invalid| {
        invalid.occurrence == NameOccurrence::AliasTarget
            && class.is_none_or(|class| invalid.class == class)
            && invalid.span.file == alias.span.file
            && alias.span.start.offset <= invalid.span.start.offset
            && invalid.span.end.offset <= alias.span.end.offset
    })
}
