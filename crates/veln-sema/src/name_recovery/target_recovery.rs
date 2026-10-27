use std::collections::{BTreeSet, HashMap, HashSet};

use veln_ast::{SurfaceModule, UseDecl};

use super::{import_classification, record_import_classification_work};

pub(super) fn recovered_target_names(
    module: &SurfaceModule,
    recovery_candidates: &[bool],
) -> Vec<Option<String>> {
    let targets = recovery_target_index(module);
    module
        .uses
        .iter()
        .enumerate()
        .map(|(index, use_decl)| {
            if !recovery_candidates[index] {
                return None;
            }
            record_import_classification_work();
            targets.get(&use_decl.name).cloned().flatten()
        })
        .collect()
}

fn recovery_target_index(module: &SurfaceModule) -> HashMap<String, Option<String>> {
    let mut targets = HashMap::<String, Option<String>>::new();
    for identity in declared_module_identities(module) {
        for written in invalid_cased_target_spellings(&identity) {
            record_recovery_target(&mut targets, written, &identity);
        }
    }
    targets
}

fn declared_module_identities(module: &SurfaceModule) -> BTreeSet<String> {
    module
        .uses
        .iter()
        .map(|declaration| &declaration.module_name)
        .chain(
            module
                .aliases
                .iter()
                .map(|declaration| &declaration.module_name),
        )
        .chain(
            module
                .effects
                .iter()
                .map(|declaration| &declaration.module_name),
        )
        .chain(
            module
                .handlers
                .iter()
                .map(|declaration| &declaration.module_name),
        )
        .chain(
            module
                .types
                .iter()
                .map(|declaration| &declaration.module_name),
        )
        .chain(
            module
                .schemas
                .iter()
                .map(|declaration| &declaration.module_name),
        )
        .chain(
            module
                .module_values
                .iter()
                .map(|declaration| &declaration.module_name),
        )
        .chain(
            module
                .functions
                .iter()
                .map(|declaration| &declaration.module_name),
        )
        .filter_map(Option::as_ref)
        .cloned()
        .collect()
}

fn invalid_cased_target_spellings(identity: &str) -> Vec<String> {
    let segments = identity.split("::").collect::<Vec<_>>();
    let mut spellings = Vec::new();
    for (index, segment) in segments.iter().copied().enumerate() {
        record_import_classification_work();
        let Some(first) = segment.as_bytes().first().copied() else {
            continue;
        };
        if !first.is_ascii_lowercase() {
            continue;
        }
        let mut written = segments.clone();
        let mut repaired = String::with_capacity(segment.len());
        repaired.push((first as char).to_ascii_uppercase());
        repaired.push_str(&segment[1..]);
        written[index] = &repaired;
        spellings.push(written.join("::"));
    }
    spellings
}

fn record_recovery_target(
    targets: &mut HashMap<String, Option<String>>,
    written: String,
    identity: &str,
) {
    targets
        .entry(written)
        .and_modify(|selected| {
            if selected.as_deref() != Some(identity) {
                *selected = None;
            }
        })
        .or_insert_with(|| Some(identity.to_owned()));
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
