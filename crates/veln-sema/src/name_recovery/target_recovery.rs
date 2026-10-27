use std::collections::{BTreeSet, HashSet};

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
        work.declared_identity_bytes += targets.iter().map(String::len).sum::<usize>();
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
            let corrected = corrected_target_spelling(use_decl)?;
            targets.contains(&corrected).then_some(corrected)
        })
        .collect()
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

fn corrected_target_spelling(use_decl: &UseDecl) -> Option<String> {
    let written_segments = use_decl.written_name.split("::").collect::<Vec<_>>();
    let mut invalid_indices = written_segments
        .iter()
        .enumerate()
        .filter_map(|(index, segment)| {
            segment
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_uppercase)
                .then_some(index)
        });
    let invalid_index = invalid_indices.next()?;
    if invalid_indices.next().is_some() {
        return None;
    }

    let internal_segments = use_decl.name.split("::").collect::<Vec<_>>();
    let prefix_count = internal_segments
        .len()
        .checked_sub(written_segments.len())?;
    let internal_index = prefix_count + invalid_index;
    let segment = *internal_segments.get(internal_index)?;
    let first = segment.as_bytes().first().copied()?;
    if !first.is_ascii_uppercase() {
        return None;
    }
    let byte_offset = internal_segments[..internal_index]
        .iter()
        .map(|segment| segment.len() + 2)
        .sum::<usize>();
    let mut corrected = use_decl.name.clone();
    corrected.replace_range(
        byte_offset..byte_offset + 1,
        &(first as char).to_ascii_lowercase().to_string(),
    );
    record_recovery_target_work(|work| work.corrected_bytes += corrected.len());
    Some(corrected)
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
