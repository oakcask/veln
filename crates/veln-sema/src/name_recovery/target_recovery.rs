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
    let mut identities = HashSet::new();
    extend_import_and_callable_identities(&mut identities, module);
    extend_type_and_data_identities(&mut identities, module);
    identities
}

fn extend_import_and_callable_identities<'a>(
    identities: &mut HashSet<(Option<&'a String>, &'a String)>,
    module: &'a SurfaceModule,
) {
    extend_declared_module_identities(
        identities,
        module.uses.iter().map(|declaration| {
            (
                declaration.importing_package.as_ref(),
                declaration.module_name.as_ref(),
            )
        }),
    );
    extend_declared_module_identities(
        identities,
        module.aliases.iter().map(|declaration| {
            (
                declaration.package_name.as_ref(),
                declaration.module_name.as_ref(),
            )
        }),
    );
    extend_declared_module_identities(
        identities,
        module.effects.iter().map(|declaration| {
            (
                declaration.package_name.as_ref(),
                declaration.module_name.as_ref(),
            )
        }),
    );
    extend_declared_module_identities(
        identities,
        module.handlers.iter().map(|declaration| {
            (
                declaration.package_name.as_ref(),
                declaration.module_name.as_ref(),
            )
        }),
    );
}

fn extend_type_and_data_identities<'a>(
    identities: &mut HashSet<(Option<&'a String>, &'a String)>,
    module: &'a SurfaceModule,
) {
    extend_declared_module_identities(
        identities,
        module.types.iter().map(|declaration| {
            (
                declaration.package_name.as_ref(),
                declaration.module_name.as_ref(),
            )
        }),
    );
    extend_declared_module_identities(
        identities,
        module.schemas.iter().map(|declaration| {
            (
                declaration.package_name.as_ref(),
                declaration.module_name.as_ref(),
            )
        }),
    );
    extend_declared_module_identities(
        identities,
        module.module_values.iter().map(|declaration| {
            (
                declaration.package_name.as_ref(),
                declaration.module_name.as_ref(),
            )
        }),
    );
    extend_declared_module_identities(
        identities,
        module.functions.iter().map(|declaration| {
            (
                declaration.package_name.as_ref(),
                declaration.module_name.as_ref(),
            )
        }),
    );
}

fn extend_declared_module_identities<'a>(
    identities: &mut HashSet<(Option<&'a String>, &'a String)>,
    declarations: impl IntoIterator<Item = (Option<&'a String>, Option<&'a String>)>,
) {
    identities.extend(
        declarations
            .into_iter()
            .filter_map(|(package, name)| name.map(|name| (package, name))),
    );
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
