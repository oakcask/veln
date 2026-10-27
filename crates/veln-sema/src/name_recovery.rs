use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use veln_ast::{NameClass, NameOccurrence, SurfaceModule, UseDecl};

mod alias_validity;
mod classification;
mod instrumentation;
mod selection_helpers;

#[cfg(test)]
use alias_validity::use_decl_is_quarantined;
pub(crate) use alias_validity::{
    InvalidAliasTargetIndex, public_alias_has_invalid_target_leaf,
    use_decl_contributes_bare_exports, use_decl_has_reserved_alias, use_decl_is_recovery_candidate,
};
use classification::{
    duplicate_diagnostic_origins, source_ordered_import_indices, source_ordered_use_indices,
};
#[cfg(test)]
pub(crate) use instrumentation::NormalImportSelectionWork;
#[cfg(test)]
pub(crate) use instrumentation::{
    import_classification_builds, import_classification_work, normal_import_selection_work,
    reset_import_classification_work, reset_normal_import_selection_work,
};
use instrumentation::{
    record_import_classification_build, record_import_classification_work,
    record_normal_import_selection_work,
};
pub(crate) use selection_helpers::{normal_use_decls, use_decl_exact_paths};

type ImportBindingKey = (Option<String>, String);
type ImportTargetKey = (Option<String>, String, Option<String>);

struct NormalUseGroups {
    quarantined: Vec<bool>,
    invalid_module_segments: Vec<bool>,
    invalid_import_locations: HashSet<(String, usize, usize)>,
    duplicate_targets: HashMap<ImportTargetKey, Vec<usize>>,
    qualifiers: HashMap<ImportBindingKey, (usize, bool)>,
    exact_paths: HashMap<ImportBindingKey, Vec<usize>>,
}

pub(crate) struct ImportClassification {
    indices: HashMap<usize, usize>,
    #[cfg(test)]
    invalid_module_segments: Vec<bool>,
    #[cfg(test)]
    duplicate_targets: Vec<bool>,
    recovery_candidates: Vec<bool>,
    invalid_import_locations: HashSet<(String, usize, usize)>,
    recovery_names_by_file: HashSet<(String, String)>,
    quarantined: Vec<bool>,
    colliding_qualifiers: HashSet<ImportBindingKey>,
    blocked_exact_paths: HashSet<ImportBindingKey>,
    source_order: Vec<usize>,
    duplicate_target_origins: Vec<Option<usize>>,
    duplicate_qualifier_origins: Vec<Option<usize>>,
}

thread_local! {
    static IMPORT_CLASSIFICATION_SCOPES: RefCell<Vec<(usize, Rc<ImportClassification>)>> =
        const { RefCell::new(Vec::new()) };
}

pub(crate) fn with_import_classification<T>(
    module: &SurfaceModule,
    operation: impl FnOnce() -> T,
) -> T {
    let key = module as *const SurfaceModule as usize;
    let existing = IMPORT_CLASSIFICATION_SCOPES.with(|scopes| {
        scopes
            .borrow()
            .iter()
            .rev()
            .find_map(|(candidate, classification)| {
                (*candidate == key).then(|| Rc::clone(classification))
            })
    });
    if existing.is_some() {
        return operation();
    }
    let classification = Rc::new(NormalUseGroups::collect(module));
    IMPORT_CLASSIFICATION_SCOPES.with(|scopes| {
        scopes.borrow_mut().push((key, classification));
    });
    struct ScopeGuard(usize);
    impl Drop for ScopeGuard {
        fn drop(&mut self) {
            IMPORT_CLASSIFICATION_SCOPES.with(|scopes| {
                let popped = scopes.borrow_mut().pop().map(|(key, _)| key);
                debug_assert_eq!(popped, Some(self.0));
            });
        }
    }
    let _guard = ScopeGuard(key);
    operation()
}

pub(crate) fn import_classification(module: &SurfaceModule) -> Rc<ImportClassification> {
    let key = module as *const SurfaceModule as usize;
    IMPORT_CLASSIFICATION_SCOPES
        .with(|scopes| {
            scopes
                .borrow()
                .iter()
                .rev()
                .find_map(|(candidate, classification)| {
                    (*candidate == key).then(|| Rc::clone(classification))
                })
        })
        .unwrap_or_else(|| Rc::new(NormalUseGroups::collect(module)))
}

impl ImportClassification {
    fn index(&self, use_decl: &UseDecl) -> Option<usize> {
        self.indices
            .get(&(use_decl as *const UseDecl as usize))
            .copied()
    }

    #[cfg(test)]
    fn has_invalid_module_segment(&self, use_decl: &UseDecl) -> bool {
        self.index(use_decl)
            .is_some_and(|index| self.invalid_module_segments[index])
    }

    #[cfg(test)]
    fn has_duplicate_target(&self, use_decl: &UseDecl) -> bool {
        self.index(use_decl)
            .is_some_and(|index| self.duplicate_targets[index])
    }

    fn is_recovery_candidate(&self, use_decl: &UseDecl) -> bool {
        self.index(use_decl)
            .is_some_and(|index| self.recovery_candidates[index])
    }

    pub(crate) fn source_order(&self) -> &[usize] {
        &self.source_order
    }

    pub(crate) fn duplicate_target_origin(&self, index: usize) -> Option<usize> {
        self.duplicate_target_origins[index]
    }

    pub(crate) fn duplicate_qualifier_origin(&self, index: usize) -> Option<usize> {
        self.duplicate_qualifier_origins[index]
    }

    pub(crate) fn invalid_path_repeats_recovery_alias(
        &self,
        invalid: &veln_ast::InvalidName,
    ) -> bool {
        record_import_classification_work();
        if self.invalid_import_locations.contains(&(
            invalid.span.file.as_str().to_owned(),
            invalid.span.start.offset,
            invalid.span.end.offset,
        )) {
            return false;
        }
        self.recovery_names_by_file
            .contains(&(invalid.span.file.as_str().to_owned(), invalid.name.clone()))
    }
}

impl NormalUseGroups {
    fn collect(module: &SurfaceModule) -> ImportClassification {
        record_import_classification_build();
        let use_order = source_ordered_use_indices(module);
        let invalid_module_spans = invalid_module_spans_by_file(module);
        let mut groups = Self {
            quarantined: vec![false; module.uses.len()],
            invalid_module_segments: vec![false; module.uses.len()],
            invalid_import_locations: HashSet::new(),
            duplicate_targets: HashMap::new(),
            qualifiers: HashMap::new(),
            exact_paths: HashMap::new(),
        };
        groups.classify_invalid_module_segments(module, &use_order, &invalid_module_spans);
        for (index, use_decl) in module.uses.iter().enumerate() {
            groups.record(index, use_decl);
        }
        groups.finish(module)
    }

    fn record(&mut self, index: usize, use_decl: &UseDecl) {
        record_import_classification_work();
        let reserved = use_decl_has_reserved_alias(use_decl);
        self.quarantined[index] =
            use_decl.recovered || reserved || self.invalid_module_segments[index];
        if use_decl.origin == veln_ast::UseOrigin::Source && !use_decl.recovered && !reserved {
            let group = self
                .qualifiers
                .entry((use_decl.module_name.clone(), use_decl.alias.clone()))
                .or_default();
            group.0 += 1;
            group.1 |= use_decl.has_explicit_alias();
        }
        if use_decl.origin == veln_ast::UseOrigin::Source && !use_decl.recovered {
            self.duplicate_targets
                .entry((
                    use_decl.module_name.clone(),
                    use_decl.written_name.clone(),
                    use_decl.package.clone(),
                ))
                .or_default()
                .push(index);
            for path in use_decl_exact_paths(use_decl) {
                self.exact_paths
                    .entry((use_decl.module_name.clone(), path))
                    .or_default()
                    .push(index);
            }
        }
    }

    fn classify_invalid_module_segments(
        &mut self,
        module: &SurfaceModule,
        source_order: &[usize],
        invalid_module_spans: &BTreeMap<String, Vec<(usize, usize)>>,
    ) {
        let mut cursors = HashMap::<&str, usize>::new();
        for &index in source_order {
            let use_decl = &module.uses[index];
            let file = use_decl.span.file.as_str();
            let Some(spans) = invalid_module_spans.get(file) else {
                continue;
            };
            let cursor = cursors.entry(file).or_default();
            loop {
                record_import_classification_work();
                if *cursor >= spans.len() {
                    break;
                }
                let (start, end) = spans[*cursor];
                record_import_classification_work();
                if start < use_decl.span.start.offset {
                    *cursor += 1;
                    continue;
                }
                record_import_classification_work();
                if start >= use_decl.span.end.offset {
                    break;
                }
                record_import_classification_work();
                if end <= use_decl.span.end.offset {
                    self.invalid_module_segments[index] = true;
                    self.invalid_import_locations
                        .insert((file.to_owned(), start, end));
                }
                *cursor += 1;
            }
        }
    }

    fn finish(mut self, module: &SurfaceModule) -> ImportClassification {
        let repeated_targets = self.mark_repeated_targets(module.uses.len());
        let blocked_exact_paths =
            blocked_exact_path_keys(std::mem::take(&mut self.exact_paths), &self.quarantined);
        let colliding_qualifiers = colliding_binding_keys(std::mem::take(&mut self.qualifiers));
        let source_order = source_ordered_import_indices(module);
        let (duplicate_target_origins, duplicate_qualifier_origins) =
            duplicate_diagnostic_origins(module, &source_order);
        let recovery_candidates =
            self.recovery_candidates(module, &repeated_targets, &colliding_qualifiers);
        let recovery_names_by_file = recovery_names_by_file(module, &recovery_candidates);
        ImportClassification {
            indices: import_indices(module),
            #[cfg(test)]
            invalid_module_segments: self.invalid_module_segments,
            #[cfg(test)]
            duplicate_targets: repeated_targets,
            recovery_candidates,
            invalid_import_locations: self.invalid_import_locations,
            recovery_names_by_file,
            quarantined: self.quarantined,
            colliding_qualifiers,
            blocked_exact_paths,
            source_order,
            duplicate_target_origins,
            duplicate_qualifier_origins,
        }
    }

    fn mark_repeated_targets(&mut self, use_count: usize) -> Vec<bool> {
        let mut repeated = vec![false; use_count];
        for indices in std::mem::take(&mut self.duplicate_targets).into_values() {
            if indices.len() < 2 {
                continue;
            }
            for index in indices {
                repeated[index] = true;
                self.quarantined[index] = true;
            }
        }
        repeated
    }

    fn recovery_candidates(
        &self,
        module: &SurfaceModule,
        repeated_targets: &[bool],
        colliding_qualifiers: &HashSet<ImportBindingKey>,
    ) -> Vec<bool> {
        module
            .uses
            .iter()
            .enumerate()
            .map(|(index, use_decl)| {
                let qualifier = (use_decl.module_name.clone(), use_decl.alias.clone());
                self.invalid_module_segments[index]
                    && !use_decl.recovered
                    && !use_decl_has_reserved_alias(use_decl)
                    && !repeated_targets[index]
                    && !colliding_qualifiers.contains(&qualifier)
            })
            .collect()
    }
}

fn import_indices(module: &SurfaceModule) -> HashMap<usize, usize> {
    module
        .uses
        .iter()
        .enumerate()
        .map(|(index, use_decl)| (use_decl as *const UseDecl as usize, index))
        .collect()
}

fn recovery_names_by_file(
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

fn prefix_bucket_offsets(counts: &mut [usize]) {
    let mut next = 0;
    for count in counts {
        let start = next;
        next += *count;
        *count = start;
    }
}

fn colliding_binding_keys(
    groups: HashMap<ImportBindingKey, (usize, bool)>,
) -> HashSet<ImportBindingKey> {
    groups
        .into_iter()
        .filter_map(|(key, (count, has_explicit))| (count > 1 && has_explicit).then_some(key))
        .collect()
}

fn blocked_exact_path_keys(
    groups: HashMap<ImportBindingKey, Vec<usize>>,
    quarantined: &[bool],
) -> HashSet<ImportBindingKey> {
    groups
        .into_iter()
        .filter_map(|(key, indices)| (indices.len() != 1 || quarantined[indices[0]]).then_some(key))
        .collect()
}

fn invalid_module_spans_by_file(module: &SurfaceModule) -> BTreeMap<String, Vec<(usize, usize)>> {
    let mut spans = BTreeMap::<String, Vec<(usize, usize)>>::new();
    for invalid in &module.invalid_names {
        record_import_classification_work();
        if invalid.class == NameClass::Module
            && matches!(
                invalid.occurrence,
                NameOccurrence::PathSegment | NameOccurrence::Declaration
            )
        {
            spans
                .entry(invalid.span.file.as_str().to_owned())
                .or_default()
                .push((invalid.span.start.offset, invalid.span.end.offset));
        }
    }
    for file_spans in spans.values_mut() {
        radix_sort_offset_pairs(file_spans);
    }
    spans
}

fn radix_sort_offset_pairs(spans: &mut Vec<(usize, usize)>) {
    if spans.len() < 2 {
        return;
    }
    let mut indices = (0..spans.len()).collect::<Vec<_>>();
    let mut scratch = vec![0usize; spans.len()];
    for use_end in [true, false] {
        for shift in (0..usize::BITS).step_by(8) {
            let mut counts = [0usize; 256];
            for &index in &indices {
                record_import_classification_work();
                let offset = if use_end {
                    spans[index].1
                } else {
                    spans[index].0
                };
                counts[(offset >> shift) as u8 as usize] += 1;
            }
            prefix_bucket_offsets(&mut counts);
            for &index in &indices {
                record_import_classification_work();
                let offset = if use_end {
                    spans[index].1
                } else {
                    spans[index].0
                };
                let bucket = (offset >> shift) as u8 as usize;
                scratch[counts[bucket]] = index;
                counts[bucket] += 1;
            }
            std::mem::swap(&mut indices, &mut scratch);
        }
    }
    let mut original = std::mem::take(spans)
        .into_iter()
        .map(Some)
        .collect::<Vec<_>>();
    spans.extend(
        indices
            .into_iter()
            .map(|index| original[index].take().expect("radix index is unique")),
    );
}

pub(crate) fn normal_imported_use_for_path<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let classification = import_classification(module);
    normal_imported_use_for_path_with_classification(
        module,
        segments,
        current_module,
        &classification,
    )
}

#[derive(Clone, Copy)]
enum IndexedUseDecl {
    Unique(usize),
    Ambiguous,
}

pub(crate) struct NormalImportSelectionIndex<'a> {
    module: &'a SurfaceModule,
    classification: Rc<ImportClassification>,
    exact_paths: HashMap<ImportBindingKey, usize>,
    qualifiers: HashMap<ImportBindingKey, IndexedUseDecl>,
}

impl<'a> NormalImportSelectionIndex<'a> {
    pub(crate) fn new(module: &'a SurfaceModule) -> Self {
        record_normal_import_selection_work(|work| work.builds += 1);
        let classification = import_classification(module);
        let mut exact_paths = HashMap::new();
        let mut qualifiers = HashMap::new();
        for (index, use_decl) in module.uses.iter().enumerate() {
            record_normal_import_selection_work(|work| work.build_import_visits += 1);
            if classification.quarantined[index] {
                continue;
            }
            for path in use_decl_exact_paths(use_decl) {
                record_normal_import_selection_work(|work| work.build_index_entries += 1);
                exact_paths
                    .entry((use_decl.module_name.clone(), path))
                    .or_insert(index);
            }
            if simple_import_alias_matches(use_decl, &use_decl.alias) {
                record_normal_import_selection_work(|work| work.build_index_entries += 1);
                qualifiers
                    .entry((use_decl.module_name.clone(), use_decl.alias.clone()))
                    .and_modify(|candidate| *candidate = IndexedUseDecl::Ambiguous)
                    .or_insert(IndexedUseDecl::Unique(index));
            }
        }
        Self {
            module,
            classification,
            exact_paths,
            qualifiers,
        }
    }

    pub(crate) fn imported_use_for_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<&'a UseDecl> {
        record_normal_import_selection_work(|work| work.lookups += 1);
        let key = (current_module.map(str::to_string), segments.join("::"));
        record_normal_import_selection_work(|work| work.lookup_index_probes += 1);
        if self.classification.blocked_exact_paths.contains(&key) {
            return None;
        }
        record_normal_import_selection_work(|work| work.lookup_index_probes += 1);
        if let Some(index) = self.exact_paths.get(&key) {
            return self.module.uses.get(*index);
        }
        record_normal_import_selection_work(|work| work.lookup_index_probes += 1);
        if self.classification.colliding_qualifiers.contains(&key) {
            return None;
        }
        record_normal_import_selection_work(|work| work.lookup_index_probes += 1);
        match self.qualifiers.get(&key) {
            Some(IndexedUseDecl::Unique(index)) => self.module.uses.get(*index),
            Some(IndexedUseDecl::Ambiguous) | None => None,
        }
    }
}

fn normal_imported_use_for_path_with_classification<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
    classification: &ImportClassification,
) -> Option<&'a UseDecl> {
    let module_path = segments.join("::");
    let key = (current_module.map(str::to_string), module_path.clone());
    if classification.blocked_exact_paths.contains(&key) {
        return None;
    }
    if let Some(exact) = module
        .uses
        .iter()
        .enumerate()
        .find_map(|(index, use_decl)| {
            record_import_classification_work();
            (!use_decl.recovered
                && use_decl_matches_exact_import_path(use_decl, &module_path, current_module)
                && !classification.quarantined[index])
                .then_some(use_decl)
        })
    {
        return Some(exact);
    }
    if classification.colliding_qualifiers.contains(&key) {
        return None;
    }
    let mut qualified = module
        .uses
        .iter()
        .enumerate()
        .filter(|(index, use_decl)| {
            record_import_classification_work();
            !classification.quarantined[*index]
                && use_decl.module_name.as_deref() == current_module
                && simple_import_alias_matches(use_decl, &module_path)
        })
        .map(|(_, use_decl)| use_decl);
    let first = qualified.next()?;
    qualified.next().is_none().then_some(first)
}

pub(crate) fn select_use_decl_for_path<'a>(
    uses: impl Iterator<Item = &'a UseDecl>,
    module_path: &str,
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let candidates = uses
        .filter(|use_decl| use_decl.module_name.as_deref() == current_module)
        .collect::<Vec<_>>();
    let mut exact = candidates.iter().copied().filter(|use_decl| {
        use_decl_matches_exact_import_path(use_decl, module_path, current_module)
    });
    if let Some(first) = exact.next() {
        return exact.next().is_none().then_some(first);
    }
    let mut qualified = candidates
        .into_iter()
        .filter(|use_decl| simple_import_alias_matches(use_decl, module_path));
    let first = qualified.next()?;
    qualified.next().is_none().then_some(first)
}

pub(crate) fn schema_composition_imported_use_for_path<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let classification = import_classification(module);
    normal_imported_use_for_path_with_classification(
        module,
        segments,
        current_module,
        &classification,
    )
    .or_else(|| {
        let module_path = segments.join("::");
        let key = (current_module.map(str::to_string), module_path.clone());
        if classification.colliding_qualifiers.contains(&key) {
            return None;
        }
        let mut candidates = module
            .uses
            .iter()
            .enumerate()
            .filter(|(index, use_decl)| {
                record_import_classification_work();
                use_decl.package.is_none()
                    && use_decl.module_name.as_deref() == current_module
                    && !classification.quarantined[*index]
                    && !use_decl.has_explicit_alias()
                    && use_decl.alias == module_path
            })
            .map(|(_, use_decl)| use_decl);
        let candidate = candidates.next()?;
        candidates.next().is_none().then_some(candidate)
    })
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

pub(crate) fn use_decl_matches_import_path(
    use_decl: &UseDecl,
    module_path: &str,
    current_module: Option<&str>,
) -> bool {
    use_decl_matches_exact_import_path(use_decl, module_path, current_module)
        || (use_decl.module_name.as_deref() == current_module
            && simple_import_alias_matches(use_decl, module_path))
}

pub(crate) fn use_decl_matches_exact_import_path(
    use_decl: &UseDecl,
    module_path: &str,
    current_module: Option<&str>,
) -> bool {
    if use_decl.module_name.as_deref() != current_module {
        return false;
    }
    use_decl_exact_paths(use_decl)
        .iter()
        .any(|path| path == module_path)
}

fn simple_import_alias_matches(use_decl: &UseDecl, module_path: &str) -> bool {
    use_decl.alias == module_path
        && (use_decl.has_explicit_alias()
            || use_decl.package.is_some()
            || !use_decl.name.contains("::")
            || use_decl.origin == veln_ast::UseOrigin::ImplicitStandardPrelude)
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
mod tests;
