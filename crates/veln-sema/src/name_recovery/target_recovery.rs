use std::collections::HashSet;

use veln_ast::{SurfaceModule, UseDecl};

use super::{
    import_classification, record_import_classification_work, record_recovery_target_work,
};

pub(super) fn recovered_target_names(
    module: &SurfaceModule,
    recovery_candidates: &[bool],
) -> Vec<Option<String>> {
    if !recovery_candidates.iter().any(|candidate| *candidate) {
        return vec![None; module.uses.len()];
    }
    let targets = declared_module_identities(module);
    record_recovery_target_work(|work| {
        work.declared_identity_bytes += targets.iter().map(|(_, name)| name.len()).sum::<usize>();
    });
    module
        .uses
        .iter()
        .enumerate()
        .map(|(index, use_decl)| {
            if !recovery_candidates[index] {
                return None;
            }
            record_import_classification_work();
            record_recovery_target_work(|work| {
                work.candidate_bytes += use_decl.name.len() + use_decl.written_name.len();
            });
            let corrected = use_decl.corrected_target_case()?;
            record_recovery_target_work(|work| work.corrected_bytes += corrected.len());
            let package = use_decl
                .package
                .as_ref()
                .or(use_decl.importing_package.as_ref());
            targets
                .contains(&(package, &corrected))
                .then_some(corrected)
        })
        .collect()
}

fn declared_module_identities(module: &SurfaceModule) -> HashSet<(Option<&String>, &String)> {
    module
        .uses
        .iter()
        .map(|declaration| {
            (
                declaration.importing_package.as_ref(),
                declaration.module_name.as_ref(),
            )
        })
        .chain(module.aliases.iter().map(|declaration| {
            (
                declaration.package_name.as_ref(),
                declaration.module_name.as_ref(),
            )
        }))
        .chain(module.effects.iter().map(|declaration| {
            (
                declaration.package_name.as_ref(),
                declaration.module_name.as_ref(),
            )
        }))
        .chain(module.handlers.iter().map(|declaration| {
            (
                declaration.package_name.as_ref(),
                declaration.module_name.as_ref(),
            )
        }))
        .chain(module.types.iter().map(|declaration| {
            (
                declaration.package_name.as_ref(),
                declaration.module_name.as_ref(),
            )
        }))
        .chain(module.schemas.iter().map(|declaration| {
            (
                declaration.package_name.as_ref(),
                declaration.module_name.as_ref(),
            )
        }))
        .chain(module.module_values.iter().map(|declaration| {
            (
                declaration.package_name.as_ref(),
                declaration.module_name.as_ref(),
            )
        }))
        .chain(module.functions.iter().map(|declaration| {
            (
                declaration.package_name.as_ref(),
                declaration.module_name.as_ref(),
            )
        }))
        .filter_map(|(package, name)| name.map(|name| (package, name)))
        .collect()
}

pub(crate) fn quarantined_use_decls(module: &SurfaceModule) -> Vec<UseDecl> {
    let classification = import_classification(module);
    module
        .uses
        .iter()
        .filter(|use_decl| classification.is_recovery_candidate(use_decl))
        .map(|use_decl| {
            let recovered_target = classification
                .recovered_target_name(use_decl)
                .map(str::to_string);
            let mut recovered = use_decl.clone();
            if let Some(target) = recovered_target {
                recovered.name = target;
            }
            recovered
        })
        .collect()
}

pub(crate) fn recovered_import_target_name(
    module: &SurfaceModule,
    use_decl: &UseDecl,
) -> Option<String> {
    import_classification(module)
        .recovered_target_name(use_decl)
        .map(str::to_string)
}

pub(super) fn recovery_names_by_file(
    module: &SurfaceModule,
    recovery_candidates: &[bool],
) -> HashSet<(String, String)> {
    let mut names = HashSet::new();
    for (index, use_decl) in module.uses.iter().enumerate() {
        if !recovery_candidates[index] {
            continue;
        }
        let file = use_decl.span.file.as_str().to_owned();
        names.insert((file.clone(), use_decl.alias.clone()));
        let leaf = use_decl
            .name
            .rsplit("::")
            .next()
            .unwrap_or(use_decl.name.as_str())
            .to_owned();
        names.insert((file, leaf));
    }
    names
}
