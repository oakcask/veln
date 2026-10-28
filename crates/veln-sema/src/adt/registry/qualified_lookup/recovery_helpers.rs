use super::*;

pub(super) fn normalized_qualified_segments(segments: &[String]) -> String {
    let byte_len =
        segments.iter().map(String::len).sum::<usize>() + segments.len().saturating_sub(1) * 2;
    let mut normalized = String::with_capacity(byte_len);
    for (index, segment) in segments.iter().enumerate() {
        if index > 0 {
            normalized.push_str("::");
        }
        if let Some(first) = segment.as_bytes().first().copied() {
            if first.is_ascii() {
                normalized.push((first as char).to_ascii_lowercase());
                normalized.push_str(&segment[1..]);
            } else {
                normalized.push_str(segment);
            }
        }
    }
    normalized
}

pub(super) fn normalized_qualified_path(path: &str) -> String {
    normalized_qualified_segments(&path.split("::").map(str::to_string).collect::<Vec<_>>())
}

pub(super) fn is_single_initial_recovery(written: &[String], actual: &str) -> bool {
    let mut actual = actual.split("::");
    let mut recoveries = 0;
    for written in written {
        let Some(actual) = actual.next() else {
            return false;
        };
        if written == actual {
            continue;
        }
        let Some(written_first) = written.as_bytes().first().copied() else {
            return false;
        };
        let Some(actual_first) = actual.as_bytes().first().copied() else {
            return false;
        };
        if !written_first.is_ascii_uppercase()
            || actual_first != written_first.to_ascii_lowercase()
            || written.get(1..) != actual.get(1..)
        {
            return false;
        }
        recoveries += 1;
    }
    actual.next().is_none() && recoveries == 1
}

pub(super) fn fallback_recovery_modules(
    qualifiers: &[String],
    current_module: Option<&str>,
    uses: &[UseDecl],
) -> BTreeMap<String, QualifiedImportModules> {
    let mut candidates = BTreeMap::<String, QualifiedImportModules>::new();
    for use_decl in uses {
        for path in crate::name_recovery::use_decl_exact_paths(use_decl) {
            crate::adt::lookup_validation::record_qualified_path_work(path.len());
            if is_single_initial_recovery(qualifiers, &path) {
                candidates.entry(path).or_default().exact.insert({
                    let target = use_decl.resolved_target();
                    (target.package, target.module)
                });
            }
        }
        crate::adt::lookup_validation::record_qualified_path_work(use_decl.alias.len());
        if is_single_initial_recovery(qualifiers, &use_decl.alias)
            && crate::name_recovery::use_decl_matches_implicit_import_alias(
                use_decl,
                &use_decl.alias,
                current_module,
            )
        {
            candidates
                .entry(use_decl.alias.clone())
                .or_default()
                .implicit
                .insert({
                    let target = use_decl.resolved_target();
                    (target.package, target.module)
                });
        }
    }
    candidates
}

pub(super) fn merge_recovered_identity(
    selected: &mut Option<QualifiedSourceIdentity>,
    identity: Option<QualifiedSourceIdentity>,
) -> bool {
    let Some(identity) = identity else {
        return false;
    };
    if identity == QualifiedSourceIdentity::Ambiguous
        || selected
            .as_ref()
            .is_some_and(|selected| selected != &identity)
    {
        return true;
    }
    *selected = Some(identity);
    false
}
