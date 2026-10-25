use std::collections::{BTreeMap, BTreeSet};

use veln_ast::{NameClass, NameOccurrence, PublicAlias, SurfaceModule, UseDecl};

pub(crate) struct InvalidAliasTargetIndex {
    spans_by_file: BTreeMap<String, Vec<(usize, usize)>>,
}

impl InvalidAliasTargetIndex {
    pub(crate) fn new(module: &SurfaceModule) -> Self {
        let mut spans_by_file = BTreeMap::<String, Vec<_>>::new();
        for invalid in &module.invalid_names {
            if invalid.occurrence == NameOccurrence::AliasTarget {
                spans_by_file
                    .entry(invalid.span.file.as_str().to_owned())
                    .or_default()
                    .push((invalid.span.start.offset, invalid.span.end.offset));
            }
        }
        for spans in spans_by_file.values_mut() {
            spans.sort_unstable_by_key(|(start, _)| *start);
        }
        Self { spans_by_file }
    }

    pub(crate) fn contains(&self, alias: &PublicAlias) -> bool {
        let Some(spans) = self.spans_by_file.get(alias.span.file.as_str()) else {
            return false;
        };
        let first = spans.partition_point(|(start, _)| *start < alias.span.start.offset);
        spans.get(first).is_some_and(|(start, end)| {
            *start < alias.span.end.offset && *end <= alias.span.end.offset
        })
    }
}

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

pub(crate) fn use_decl_is_quarantined_with_collisions(
    module: &SurfaceModule,
    use_decl: &UseDecl,
    collisions: &ImportCollisionIndex,
) -> bool {
    use_decl.syntax_recovered
        || use_decl.resolution_quarantined
        || use_decl_has_invalid_casing(module, use_decl)
        || (use_decl.origin == veln_ast::UseOrigin::Source
            && use_decl.explicit_alias
            && use_decl.alias == "prelude")
        || collisions.contains_target(use_decl)
}

type ImportTargetKey = (Option<String>, Option<String>, String);
type ImportQualifierKey = (Option<String>, String);

/// Precomputed target and qualifier collisions for source imports.
#[doc(hidden)]
pub struct ImportCollisionIndex {
    targets: BTreeMap<ImportTargetKey, usize>,
    qualifiers: BTreeMap<ImportQualifierKey, usize>,
    duplicate_targets_with_competing_qualifiers: BTreeSet<ImportTargetKey>,
}

impl ImportCollisionIndex {
    /// Builds the collision facts for one lowered module.
    #[doc(hidden)]
    pub(crate) fn new(module: &SurfaceModule) -> Self {
        let mut index = Self::from_eligible_uses(&module.uses);
        let competing_qualifiers = module
            .uses
            .iter()
            .filter(|candidate| {
                record_normal_use_relation_candidate_inspection();
                !candidate.syntax_recovered
                    && !use_decl_has_invalid_casing(module, candidate)
                    && !(candidate.origin == veln_ast::UseOrigin::Source
                        && candidate.explicit_alias
                        && candidate.alias == "prelude")
                    && !index.contains_target(candidate)
                    && !index.contains_qualifier(candidate)
                    && use_decl_has_import_qualifier(candidate)
            })
            .map(|candidate| (candidate.module_name.clone(), candidate.alias.clone()))
            .collect::<BTreeSet<_>>();
        index.duplicate_targets_with_competing_qualifiers = index
            .targets
            .iter()
            .filter(|((module_name, _, written_name), count)| {
                **count > 1
                    && competing_qualifiers.contains(&(module_name.clone(), written_name.clone()))
            })
            .map(|(target, _)| target.clone())
            .collect();
        index
    }

    pub(crate) fn from_eligible_uses(uses: &[UseDecl]) -> Self {
        Self::from_eligible_use_refs(uses)
    }

    /// Builds collision facts from the provided source imports.
    #[doc(hidden)]
    pub fn from_eligible_use_refs<'a>(uses: impl IntoIterator<Item = &'a UseDecl>) -> Self {
        let mut targets = BTreeMap::new();
        let mut qualifiers = BTreeMap::new();
        for use_decl in uses.into_iter().filter(|use_decl| {
            use_decl.origin == veln_ast::UseOrigin::Source && !use_decl.syntax_recovered
        }) {
            *targets
                .entry((
                    use_decl.module_name.clone(),
                    use_decl.package.clone(),
                    use_decl.written_name.clone(),
                ))
                .or_insert(0) += 1;
            if use_decl_has_import_qualifier(use_decl) {
                *qualifiers
                    .entry((use_decl.module_name.clone(), use_decl.alias.clone()))
                    .or_insert(0) += 1;
            }
        }
        Self {
            targets,
            qualifiers,
            duplicate_targets_with_competing_qualifiers: BTreeSet::new(),
        }
    }

    /// Reports whether the declaration's target identity is duplicated.
    #[doc(hidden)]
    pub fn contains_target(&self, use_decl: &UseDecl) -> bool {
        self.targets
            .get(&(
                use_decl.module_name.clone(),
                use_decl.package.clone(),
                use_decl.written_name.clone(),
            ))
            .is_some_and(|count| *count > 1)
    }

    /// Reports whether the declaration's local qualifier is duplicated.
    #[doc(hidden)]
    pub fn contains_qualifier(&self, use_decl: &UseDecl) -> bool {
        self.qualifiers
            .get(&(use_decl.module_name.clone(), use_decl.alias.clone()))
            .is_some_and(|count| *count > 1)
    }

    fn duplicate_target_blocks_competing_qualifier(&self, use_decl: &UseDecl) -> bool {
        self.duplicate_targets_with_competing_qualifiers
            .contains(&import_target_key(use_decl))
    }
}

fn import_target_key(use_decl: &UseDecl) -> ImportTargetKey {
    (
        use_decl.module_name.clone(),
        use_decl.package.clone(),
        use_decl.written_name.clone(),
    )
}

pub(crate) fn use_decl_has_invalid_casing(module: &SurfaceModule, use_decl: &UseDecl) -> bool {
    use_decl_has_invalid_module_segment(module, use_decl)
        || module.invalid_names.iter().any(|invalid| {
            invalid.class == NameClass::Module
                && invalid.occurrence == NameOccurrence::Declaration
                && invalid.span.file == use_decl.span.file
                && use_decl.span.start.offset <= invalid.span.start.offset
                && invalid.span.end.offset <= use_decl.span.end.offset
        })
}

pub(crate) fn use_decl_has_invalid_explicit_alias(
    module: &SurfaceModule,
    use_decl: &UseDecl,
) -> bool {
    let Some(alias_span) = use_decl
        .alias_span
        .as_ref()
        .filter(|_| use_decl.explicit_alias)
    else {
        return false;
    };
    module.invalid_names.iter().any(|invalid| {
        invalid.class == NameClass::Module
            && invalid.occurrence == NameOccurrence::Declaration
            && invalid.span == *alias_span
    })
}

pub(crate) fn normal_use_decls(module: &SurfaceModule) -> Vec<UseDecl> {
    let collisions = ImportCollisionIndex::new(module);
    module
        .uses
        .iter()
        .filter(|use_decl| {
            !use_decl.syntax_recovered
                && !use_decl_has_invalid_casing(module, use_decl)
                && !(use_decl.origin == veln_ast::UseOrigin::Source
                    && use_decl.explicit_alias
                    && use_decl.alias == "prelude")
                && (!collisions.contains_target(use_decl)
                    || collisions.duplicate_target_blocks_competing_qualifier(use_decl))
        })
        .map(|use_decl| {
            let mut resolved = resolved_import_use_decl(use_decl);
            if use_decl.resolution_quarantined {
                // Preserve the exact path and qualifier as tombstones while
                // preventing bare-member visibility in downstream indexes.
                resolved.explicit_alias = true;
            } else if collisions.contains_target(use_decl) {
                // Retain duplicate exact paths as ambiguity tombstones without
                // allowing their bare or qualifier routes to participate.
                resolved.explicit_alias = true;
                resolved.alias.clear();
            }
            resolved
        })
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
    let collisions = ImportCollisionIndex::new(module);
    let candidates = module
        .uses
        .iter()
        .filter(|use_decl| {
            !use_decl.syntax_recovered
                && !use_decl_has_invalid_casing(module, use_decl)
                && !(use_decl.origin == veln_ast::UseOrigin::Source
                    && use_decl.explicit_alias
                    && use_decl.alias == "prelude")
        })
        .collect::<Vec<_>>();
    match select_from_candidates(&candidates, segments, current_module, &collisions) {
        ImportSelection::One(use_decl) => Some(use_decl),
        ImportSelection::None | ImportSelection::Ambiguous => None,
    }
}

pub(crate) enum ImportSelection<'a> {
    None,
    One(&'a UseDecl),
    Ambiguous,
}

fn unique_use<'a>(mut candidates: impl Iterator<Item = &'a UseDecl>) -> ImportSelection<'a> {
    let Some(first) = candidates.next() else {
        return ImportSelection::None;
    };
    if candidates.next().is_some() {
        ImportSelection::Ambiguous
    } else {
        ImportSelection::One(first)
    }
}

fn select_from_candidates<'a>(
    candidates: &[&'a UseDecl],
    segments: &[String],
    current_module: Option<&str>,
    collisions: &ImportCollisionIndex,
) -> ImportSelection<'a> {
    let module_path = segments.join("::");
    let exact_candidates = candidates.iter().copied().filter(|use_decl| {
        use_decl_matches_exact_import_path(use_decl, &module_path, current_module)
    });
    if exact_candidates
        .clone()
        .any(|use_decl| use_decl.resolution_quarantined)
    {
        return ImportSelection::Ambiguous;
    }
    match unique_use(exact_candidates) {
        ImportSelection::One(use_decl) => return ImportSelection::One(use_decl),
        ImportSelection::Ambiguous => return ImportSelection::Ambiguous,
        ImportSelection::None => {}
    }
    let qualifier_candidates = candidates.iter().copied().filter(|use_decl| {
        use_decl.module_name.as_deref() == current_module
            && !collisions.contains_target(use_decl)
            && simple_import_alias_matches(use_decl, &module_path)
    });
    if qualifier_candidates
        .clone()
        .any(|use_decl| use_decl.resolution_quarantined)
    {
        return ImportSelection::Ambiguous;
    }
    if qualifier_candidates
        .clone()
        .any(|use_decl| collisions.contains_qualifier(use_decl))
    {
        return ImportSelection::Ambiguous;
    }
    unique_use(qualifier_candidates)
}

pub(crate) fn select_imported_use_for_path<'a>(
    uses: &'a [UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> ImportSelection<'a> {
    let candidates = uses
        .iter()
        .filter(|use_decl| !use_decl.syntax_recovered)
        .collect::<Vec<_>>();
    let collisions = ImportCollisionIndex::from_eligible_uses(uses);
    select_from_candidates(&candidates, segments, current_module, &collisions)
}

pub(crate) fn schema_composition_imported_use_for_path<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    record_schema_composition_import_lookup(module.uses.len());
    let collisions = ImportCollisionIndex::new(module);
    let candidates = module
        .uses
        .iter()
        .filter(|use_decl| {
            record_schema_composition_quarantine_check();
            !use_decl.syntax_recovered
                && !use_decl_has_invalid_casing(module, use_decl)
                && !(use_decl.origin == veln_ast::UseOrigin::Source
                    && use_decl.explicit_alias
                    && use_decl.alias == "prelude")
        })
        .collect::<Vec<_>>();
    match select_from_candidates(&candidates, segments, current_module, &collisions) {
        ImportSelection::One(use_decl) => Some(use_decl),
        ImportSelection::Ambiguous => None,
        ImportSelection::None => {
            let module_path = segments.join("::");
            let mut candidates = module.uses.iter().filter(|use_decl| {
                record_schema_composition_quarantine_check();
                use_decl.package.is_none()
                    && use_decl.module_name.as_deref() == current_module
                    && !use_decl_is_quarantined_with_collisions(module, use_decl, &collisions)
                    && use_decl.alias == module_path
            });
            let candidate = candidates.next()?;
            candidates.next().is_none().then_some(candidate)
        }
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SchemaCompositionImportLookupWork {
    pub(crate) lookups: usize,
    pub(crate) indexed_imports: usize,
    pub(crate) quarantine_checks: usize,
}

#[cfg(test)]
thread_local! {
    static SCHEMA_COMPOSITION_IMPORT_LOOKUP_WORK: std::cell::Cell<SchemaCompositionImportLookupWork> =
        const { std::cell::Cell::new(SchemaCompositionImportLookupWork {
            lookups: 0,
            indexed_imports: 0,
            quarantine_checks: 0,
        }) };
}

#[cfg(test)]
fn record_schema_composition_import_lookup(imports: usize) {
    SCHEMA_COMPOSITION_IMPORT_LOOKUP_WORK.with(|work| {
        let mut next = work.get();
        next.lookups += 1;
        next.indexed_imports += imports;
        work.set(next);
    });
}

#[cfg(not(test))]
fn record_schema_composition_import_lookup(_imports: usize) {}

#[cfg(test)]
fn record_schema_composition_quarantine_check() {
    SCHEMA_COMPOSITION_IMPORT_LOOKUP_WORK.with(|work| {
        let mut next = work.get();
        next.quarantine_checks += 1;
        work.set(next);
    });
}

#[cfg(not(test))]
fn record_schema_composition_quarantine_check() {}

#[cfg(test)]
pub(crate) fn reset_schema_composition_import_lookup_work() {
    SCHEMA_COMPOSITION_IMPORT_LOOKUP_WORK.with(|work| {
        work.set(SchemaCompositionImportLookupWork::default());
    });
}

#[cfg(test)]
pub(crate) fn take_schema_composition_import_lookup_work() -> SchemaCompositionImportLookupWork {
    SCHEMA_COMPOSITION_IMPORT_LOOKUP_WORK
        .with(|work| work.replace(SchemaCompositionImportLookupWork::default()))
}

pub(crate) fn resolved_import_module_name(
    use_decl: &UseDecl,
    current_module: Option<&str>,
) -> String {
    if (use_decl.package.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
        || (use_decl.package.is_none()
            && current_module.is_some_and(|module| module.starts_with("std::"))))
        && !use_decl.name.starts_with("std::")
    {
        format!("std::{}", use_decl.name)
    } else {
        use_decl.name.clone()
    }
}

pub(crate) fn use_decl_matches_exact_import_path(
    use_decl: &UseDecl,
    module_path: &str,
    current_module: Option<&str>,
) -> bool {
    if use_decl.module_name.as_deref() != current_module {
        return false;
    }
    if use_decl.written_name == module_path {
        return true;
    }
    if use_decl.package.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
        && use_decl.name == module_path
    {
        return true;
    }
    if use_decl.package.is_none()
        && current_module.is_some_and(|module| module.starts_with("std::"))
        && use_decl.name == module_path
    {
        return true;
    }
    if use_decl.package.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
        && use_decl
            .name
            .strip_prefix("std::")
            .is_some_and(|package_relative| package_relative == module_path)
    {
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

fn simple_import_alias_matches(use_decl: &UseDecl, module_path: &str) -> bool {
    use_decl.alias == module_path && use_decl_has_import_qualifier(use_decl)
}

pub(crate) fn use_decl_has_import_qualifier(use_decl: &UseDecl) -> bool {
    use_decl.explicit_alias
        || use_decl.package.is_some()
        || !use_decl.name.contains("::")
        || use_decl.origin == veln_ast::UseOrigin::ImplicitStandardPrelude
}

pub(crate) fn use_decl_has_qualifier_collision(uses: &[UseDecl], use_decl: &UseDecl) -> bool {
    use_decl_has_import_qualifier(use_decl)
        && ImportCollisionIndex::from_eligible_uses(uses).contains_qualifier(use_decl)
}

pub(crate) fn use_decl_matches_implicit_import_alias(
    use_decl: &UseDecl,
    module_path: &str,
    current_module: Option<&str>,
) -> bool {
    use_decl.module_name.as_deref() == current_module
        && simple_import_alias_matches(use_decl, module_path)
}

#[cfg(test)]
thread_local! {
    static NORMAL_USE_RELATION_CANDIDATE_INSPECTIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn record_normal_use_relation_candidate_inspection() {
    NORMAL_USE_RELATION_CANDIDATE_INSPECTIONS.with(|inspections| {
        inspections.set(inspections.get() + 1);
    });
}

#[cfg(not(test))]
fn record_normal_use_relation_candidate_inspection() {}

#[cfg(test)]
pub(crate) fn reset_normal_use_relation_candidate_inspections() {
    NORMAL_USE_RELATION_CANDIDATE_INSPECTIONS.with(|inspections| inspections.set(0));
}

#[cfg(test)]
pub(crate) fn normal_use_relation_candidate_inspections() -> usize {
    NORMAL_USE_RELATION_CANDIDATE_INSPECTIONS.with(std::cell::Cell::get)
}
