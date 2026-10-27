use veln_ast::{SurfaceModule, UseDecl};

use super::{import_classification, resolved_import_module_name};

pub(crate) fn normal_use_decls(module: &SurfaceModule) -> Vec<UseDecl> {
    let classification = import_classification(module);
    module
        .uses
        .iter()
        .zip(classification.quarantined.iter())
        .filter(|(_, quarantined)| !**quarantined)
        .map(|(use_decl, _)| {
            let mut resolved = resolved_import_use_decl(use_decl);
            let qualifier = (use_decl.resolved_owner(), use_decl.alias.clone());
            if classification.colliding_qualifiers.contains(&qualifier)
                || classification.blocked_exact_paths.contains(&qualifier)
            {
                resolved.alias.clear();
            }
            resolved
        })
        .collect()
}

pub(crate) fn use_decl_exact_paths(use_decl: &UseDecl) -> Vec<String> {
    use_decl.exact_paths()
}

pub(crate) fn resolved_import_use_decl(use_decl: &UseDecl) -> UseDecl {
    let mut resolved = use_decl.clone();
    resolved.name = resolved_import_module_name(use_decl, use_decl.module_name.as_deref());
    resolved
}
