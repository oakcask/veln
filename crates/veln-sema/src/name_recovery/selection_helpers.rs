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
            let qualifier = (use_decl.module_name.clone(), use_decl.alias.clone());
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
    let mut paths = vec![use_decl.written_name.clone()];
    if is_standard_import(use_decl) {
        paths.push(use_decl.name.clone());
        for path in [use_decl.written_name.as_str(), use_decl.name.as_str()] {
            if let Some(relative) = path.strip_prefix("std::") {
                paths.push(relative.to_string());
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn is_standard_import(use_decl: &UseDecl) -> bool {
    use_decl.package.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
        || use_decl.name.starts_with("std::")
        || (use_decl.package.is_none()
            && use_decl
                .module_name
                .as_deref()
                .is_some_and(|module| module.starts_with("std::")))
}

pub(crate) fn resolved_import_use_decl(use_decl: &UseDecl) -> UseDecl {
    let mut resolved = use_decl.clone();
    resolved.name = resolved_import_module_name(use_decl, use_decl.module_name.as_deref());
    resolved
}
