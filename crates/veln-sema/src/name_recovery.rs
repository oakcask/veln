use veln_ast::{NameClass, NameOccurrence, PublicAlias, SurfaceModule, UseDecl};

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

pub(crate) fn use_decl_has_invalid_module_segment(
    module: &SurfaceModule,
    use_decl: &UseDecl,
) -> bool {
    module.invalid_names.iter().any(|invalid| {
        invalid.class == NameClass::Module
            && invalid.occurrence == NameOccurrence::PathSegment
            && invalid.span.file == use_decl.span.file
            && use_decl.span.start.offset <= invalid.span.start.offset
            && invalid.span.end.offset <= use_decl.span.end.offset
    })
}

pub(crate) fn normal_use_decls(module: &SurfaceModule) -> Vec<UseDecl> {
    module
        .uses
        .iter()
        .filter(|use_decl| !use_decl_has_invalid_module_segment(module, use_decl))
        .map(resolved_import_use_decl)
        .collect()
}

pub(crate) fn resolved_import_use_decl(use_decl: &UseDecl) -> UseDecl {
    let mut resolved = use_decl.clone();
    resolved.name = resolved_import_module_name(use_decl, use_decl.module_name.as_deref());
    resolved
}

pub(crate) fn normal_imported_use_for_path<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let module_path = segments.join("::");
    module.uses.iter().find(|use_decl| {
        !use_decl_has_invalid_module_segment(module, use_decl)
            && use_decl_matches_import_path(use_decl, &module_path, current_module)
    })
}

pub(crate) fn resolved_import_module_name(
    use_decl: &UseDecl,
    current_module: Option<&str>,
) -> String {
    if use_decl.package.is_none()
        && current_module.is_some_and(|module| module.starts_with("std::"))
        && !use_decl.name.starts_with("std::")
    {
        format!("std::{}", use_decl.name)
    } else {
        use_decl.name.clone()
    }
}

pub(crate) fn use_decl_matches_import_path(
    use_decl: &UseDecl,
    module_path: &str,
    current_module: Option<&str>,
) -> bool {
    if use_decl.module_name.as_deref() != current_module {
        return false;
    }
    if use_decl.name == module_path || use_decl.alias == module_path {
        return true;
    }
    if use_decl.package.is_some()
        || !current_module.is_some_and(|module| module.starts_with("std::"))
    {
        return false;
    }
    use_decl
        .name
        .strip_prefix("std::")
        .is_some_and(|package_relative| package_relative == module_path)
}
