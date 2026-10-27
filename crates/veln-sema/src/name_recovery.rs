use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use veln_ast::{NameClass, NameOccurrence, PublicAlias, SurfaceModule, UseDecl};

#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static IMPORT_CLASSIFICATION_WORK: Cell<usize> = const { Cell::new(0) };
    static IMPORT_CLASSIFICATION_BUILDS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
fn record_import_classification_work() {
    IMPORT_CLASSIFICATION_WORK.set(IMPORT_CLASSIFICATION_WORK.get() + 1);
}

#[cfg(not(test))]
fn record_import_classification_work() {}

#[cfg(test)]
pub(crate) fn reset_import_classification_work() {
    IMPORT_CLASSIFICATION_WORK.set(0);
    IMPORT_CLASSIFICATION_BUILDS.set(0);
}

#[cfg(test)]
pub(crate) fn import_classification_work() -> usize {
    IMPORT_CLASSIFICATION_WORK.get()
}

#[cfg(test)]
pub(crate) fn import_classification_builds() -> usize {
    IMPORT_CLASSIFICATION_BUILDS.get()
}

#[cfg(test)]
fn record_import_classification_build() {
    IMPORT_CLASSIFICATION_BUILDS.set(IMPORT_CLASSIFICATION_BUILDS.get() + 1);
}

#[cfg(not(test))]
fn record_import_classification_build() {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NormalImportSelectionWork {
    pub(crate) builds: usize,
    pub(crate) build_import_visits: usize,
    pub(crate) build_index_entries: usize,
    pub(crate) lookups: usize,
    pub(crate) lookup_index_probes: usize,
}

#[cfg(test)]
thread_local! {
    static NORMAL_IMPORT_SELECTION_WORK: Cell<NormalImportSelectionWork> =
        const { Cell::new(NormalImportSelectionWork {
            builds: 0,
            build_import_visits: 0,
            build_index_entries: 0,
            lookups: 0,
            lookup_index_probes: 0,
        }) };
}

#[cfg(test)]
fn record_normal_import_selection_work(update: impl FnOnce(&mut NormalImportSelectionWork)) {
    NORMAL_IMPORT_SELECTION_WORK.set({
        let mut work = NORMAL_IMPORT_SELECTION_WORK.get();
        update(&mut work);
        work
    });
}

#[cfg(not(test))]
fn record_normal_import_selection_work(_update: impl FnOnce(&mut NormalImportSelectionWork)) {}

#[cfg(test)]
pub(crate) fn reset_normal_import_selection_work() {
    NORMAL_IMPORT_SELECTION_WORK.set(NormalImportSelectionWork::default());
}

#[cfg(test)]
pub(crate) fn normal_import_selection_work() -> NormalImportSelectionWork {
    NORMAL_IMPORT_SELECTION_WORK.get()
}

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

#[cfg(test)]
pub(crate) fn use_decl_has_invalid_module_segment(
    module: &SurfaceModule,
    use_decl: &UseDecl,
) -> bool {
    import_classification(module).has_invalid_module_segment(use_decl)
}

#[cfg(test)]
pub(crate) fn use_decl_has_invalid_alias(module: &SurfaceModule, use_decl: &UseDecl) -> bool {
    let Some(alias_span) = &use_decl.alias_span else {
        return false;
    };
    module.invalid_names.iter().any(|invalid| {
        invalid.class == NameClass::Module
            && invalid.occurrence == NameOccurrence::Declaration
            && invalid.span == *alias_span
    })
}

#[cfg(test)]
pub(crate) fn use_decl_has_duplicate_target(module: &SurfaceModule, use_decl: &UseDecl) -> bool {
    import_classification(module).has_duplicate_target(use_decl)
}

pub(crate) fn use_decl_has_reserved_alias(use_decl: &UseDecl) -> bool {
    use_decl.origin == veln_ast::UseOrigin::Source
        && use_decl.has_explicit_alias()
        && use_decl.alias == "prelude"
}

pub(crate) fn use_decl_is_recovery_candidate(module: &SurfaceModule, use_decl: &UseDecl) -> bool {
    import_classification(module).is_recovery_candidate(use_decl)
}

#[cfg(test)]
pub(crate) fn use_decl_is_quarantined(module: &SurfaceModule, use_decl: &UseDecl) -> bool {
    use_decl.recovered
        || use_decl_has_invalid_module_segment(module, use_decl)
        || use_decl_has_invalid_alias(module, use_decl)
        || use_decl_has_reserved_alias(use_decl)
        || use_decl_has_duplicate_target(module, use_decl)
}

pub(crate) fn use_decl_contributes_bare_exports(use_decl: &UseDecl) -> bool {
    !use_decl.has_explicit_alias()
}

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
    let classification = Rc::new(NormalUseGroups::collect(module).finish(module));
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
        .unwrap_or_else(|| Rc::new(NormalUseGroups::collect(module).finish(module)))
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
    fn collect(module: &SurfaceModule) -> Self {
        record_import_classification_build();
        let invalid_module_spans = invalid_module_spans_by_file(module);
        let mut groups = Self {
            quarantined: vec![false; module.uses.len()],
            invalid_module_segments: vec![false; module.uses.len()],
            invalid_import_locations: HashSet::new(),
            duplicate_targets: HashMap::new(),
            qualifiers: HashMap::new(),
            exact_paths: HashMap::new(),
        };
        for (index, use_decl) in module.uses.iter().enumerate() {
            groups.record(index, use_decl, &invalid_module_spans);
        }
        groups
    }

    fn record(
        &mut self,
        index: usize,
        use_decl: &UseDecl,
        invalid_module_spans: &BTreeMap<String, Vec<(usize, usize)>>,
    ) {
        record_import_classification_work();
        let reserved = use_decl_has_reserved_alias(use_decl);
        self.invalid_module_segments[index] =
            use_decl_has_indexed_invalid_module_segment(invalid_module_spans, use_decl);
        if let Some(spans) = invalid_module_spans.get(use_decl.span.file.as_str()) {
            let first = spans.partition_point(|(start, _)| *start < use_decl.span.start.offset);
            for &(start, end) in spans[first..]
                .iter()
                .take_while(|(start, _)| *start < use_decl.span.end.offset)
            {
                if end <= use_decl.span.end.offset {
                    self.invalid_import_locations.insert((
                        use_decl.span.file.as_str().to_owned(),
                        start,
                        end,
                    ));
                }
            }
        }
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

    fn finish(mut self, module: &SurfaceModule) -> ImportClassification {
        let mut repeated_targets = vec![false; module.uses.len()];
        let duplicate_target_groups = std::mem::take(&mut self.duplicate_targets);
        for indices in duplicate_target_groups.into_values() {
            if indices.len() > 1 {
                for index in indices {
                    repeated_targets[index] = true;
                    self.quarantined[index] = true;
                }
            }
        }
        let blocked_exact_paths = blocked_exact_path_keys(self.exact_paths, &self.quarantined);
        let colliding_qualifiers = colliding_binding_keys(self.qualifiers);
        let source_order = source_ordered_import_indices(module);
        let (duplicate_target_origins, duplicate_qualifier_origins) =
            duplicate_diagnostic_origins(module, &source_order);
        let recovery_candidates = module
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
            .collect::<Vec<_>>();
        let mut recovery_names_by_file = HashSet::new();
        for (index, use_decl) in module.uses.iter().enumerate() {
            if recovery_candidates[index] {
                let file = use_decl.span.file.as_str().to_owned();
                recovery_names_by_file.insert((file.clone(), use_decl.alias.clone()));
                recovery_names_by_file.insert((
                    file,
                    use_decl
                        .name
                        .rsplit("::")
                        .next()
                        .unwrap_or(use_decl.name.as_str())
                        .to_owned(),
                ));
            }
        }
        ImportClassification {
            indices: module
                .uses
                .iter()
                .enumerate()
                .map(|(index, use_decl)| (use_decl as *const UseDecl as usize, index))
                .collect(),
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
}

fn source_ordered_import_indices(module: &SurfaceModule) -> Vec<usize> {
    let mut indices = module
        .uses
        .iter()
        .enumerate()
        .filter_map(|(index, use_decl)| {
            (use_decl.origin == veln_ast::UseOrigin::Source && !use_decl.recovered).then_some(index)
        })
        .collect::<Vec<_>>();
    if indices.len() < 2 {
        return indices;
    }
    let mut scratch = vec![0; indices.len()];
    for use_end in [true, false] {
        for shift in (0..usize::BITS).step_by(8) {
            radix_sort_import_offset_byte(module, &indices, &mut scratch, shift, use_end);
            std::mem::swap(&mut indices, &mut scratch);
        }
    }
    radix_sort_import_file_bytes(module, &mut indices, &mut scratch);
    indices
}

fn radix_sort_import_offset_byte(
    module: &SurfaceModule,
    indices: &[usize],
    output: &mut [usize],
    shift: u32,
    use_end: bool,
) {
    let mut counts = [0usize; 256];
    for &index in indices {
        record_import_classification_work();
        let span = &module.uses[index].span;
        let offset = if use_end {
            span.end.offset
        } else {
            span.start.offset
        };
        counts[(offset >> shift) as u8 as usize] += 1;
    }
    prefix_bucket_offsets(&mut counts);
    for &index in indices {
        record_import_classification_work();
        let span = &module.uses[index].span;
        let offset = if use_end {
            span.end.offset
        } else {
            span.start.offset
        };
        let bucket = (offset >> shift) as u8 as usize;
        output[counts[bucket]] = index;
        counts[bucket] += 1;
    }
}

fn radix_sort_import_file_bytes(
    module: &SurfaceModule,
    indices: &mut [usize],
    scratch: &mut [usize],
) {
    let Some(&first) = indices.first() else {
        return;
    };
    let first_file = &module.uses[first].span.file;
    if indices
        .iter()
        .all(|index| module.uses[*index].span.file == *first_file)
    {
        return;
    }
    let mut pending = vec![(0usize, indices.len(), 0usize)];
    while let Some((start, end, byte_index)) = pending.pop() {
        if end - start < 2 {
            continue;
        }
        let mut sizes = [0usize; 257];
        for &index in &indices[start..end] {
            record_import_classification_work();
            sizes[import_file_byte_bucket(&module.uses[index], byte_index)] += 1;
        }
        let mut positions = sizes;
        let mut next = start;
        for position in &mut positions {
            let bucket_start = next;
            next += *position;
            *position = bucket_start;
        }
        for &index in &indices[start..end] {
            record_import_classification_work();
            let bucket = import_file_byte_bucket(&module.uses[index], byte_index);
            scratch[positions[bucket]] = index;
            positions[bucket] += 1;
        }
        indices[start..end].copy_from_slice(&scratch[start..end]);
        let mut bucket_end = end;
        for bucket in (1..sizes.len()).rev() {
            let bucket_start = bucket_end - sizes[bucket];
            if bucket_end - bucket_start > 1 {
                pending.push((bucket_start, bucket_end, byte_index + 1));
            }
            bucket_end = bucket_start;
        }
    }
}

fn import_file_byte_bucket(use_decl: &UseDecl, byte_index: usize) -> usize {
    use_decl
        .span
        .file
        .as_str()
        .as_bytes()
        .get(byte_index)
        .map_or(0, |byte| usize::from(*byte) + 1)
}

fn prefix_bucket_offsets(counts: &mut [usize]) {
    let mut next = 0;
    for count in counts {
        let start = next;
        next += *count;
        *count = start;
    }
}

#[derive(Clone, Copy, Default)]
struct FirstTwo {
    first: Option<usize>,
    second: Option<usize>,
}

impl FirstTwo {
    fn push(&mut self, index: usize) {
        if self.first.is_none() {
            self.first = Some(index);
        } else if self.second.is_none() {
            self.second = Some(index);
        }
    }

    fn first_except(self, excluded: Option<usize>) -> Option<usize> {
        self.first
            .filter(|candidate| Some(*candidate) != excluded)
            .or_else(|| self.second.filter(|candidate| Some(*candidate) != excluded))
    }
}

fn duplicate_diagnostic_origins(
    module: &SurfaceModule,
    source_order: &[usize],
) -> (Vec<Option<usize>>, Vec<Option<usize>>) {
    let mut target_origins = vec![None; module.uses.len()];
    let mut qualifier_origins = vec![None; module.uses.len()];
    let mut first_targets = HashMap::<ImportTargetKey, usize>::new();
    let mut first_qualifiers = HashMap::<ImportBindingKey, FirstTwo>::new();
    let mut first_explicit_qualifiers = HashMap::<ImportBindingKey, FirstTwo>::new();
    for &index in source_order {
        record_import_classification_work();
        let use_decl = &module.uses[index];
        let target = (
            use_decl.module_name.clone(),
            use_decl.written_name.clone(),
            use_decl.package.clone(),
        );
        let duplicate_target = first_targets.get(&target).copied();
        target_origins[index] = duplicate_target;
        let qualifier = (use_decl.module_name.clone(), use_decl.alias.clone());
        let candidates = if use_decl.has_explicit_alias() {
            first_qualifiers.get(&qualifier).copied()
        } else {
            first_explicit_qualifiers.get(&qualifier).copied()
        };
        qualifier_origins[index] =
            candidates.and_then(|candidates| candidates.first_except(duplicate_target));
        first_targets.entry(target).or_insert(index);
        first_qualifiers
            .entry(qualifier.clone())
            .or_default()
            .push(index);
        if use_decl.has_explicit_alias() {
            first_explicit_qualifiers
                .entry(qualifier)
                .or_default()
                .push(index);
        }
    }
    (target_origins, qualifier_origins)
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

fn use_decl_has_indexed_invalid_module_segment(
    invalid_module_spans: &BTreeMap<String, Vec<(usize, usize)>>,
    use_decl: &UseDecl,
) -> bool {
    let Some(spans) = invalid_module_spans.get(use_decl.span.file.as_str()) else {
        return false;
    };
    record_import_classification_work();
    let first = spans.partition_point(|(start, _)| *start < use_decl.span.start.offset);
    spans[first..]
        .iter()
        .take_while(|(start, _)| {
            record_import_classification_work();
            *start < use_decl.span.end.offset
        })
        .any(|(start, end)| {
            use_decl.span.start.offset <= *start && *end <= use_decl.span.end.offset
        })
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
mod tests {
    use super::*;

    #[test]
    fn invalid_explicit_aliases_are_atomically_quarantined() {
        for alias in ["_wire", "_"] {
            let source = veln_source::SourceFile::new(
                "app.veln",
                format!("mod app\nuse transport as {alias}\n"),
            );
            let parsed = veln_syntax::parse(&source);
            assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
            let module = veln_ast::lower_surface_ast(&parsed.tree);
            assert_eq!(module.uses.len(), 1, "alias={alias}");
            assert!(use_decl_is_quarantined(&module, &module.uses[0]));
            assert!(normal_use_decls(&module).is_empty(), "alias={alias}");
        }
    }

    #[test]
    fn normal_import_classification_work_scales_linearly() {
        fn work(count: usize) -> usize {
            let mut text = String::from("mod app\n");
            for index in 0..count {
                text.push_str(&format!("use dependency{index} as alias{index}\n"));
            }
            let source = veln_source::SourceFile::new("app.veln", text);
            let parsed = veln_syntax::parse(&source);
            assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
            let module = veln_ast::lower_surface_ast(&parsed.tree);
            reset_import_classification_work();
            assert_eq!(normal_use_decls(&module).len(), count);
            import_classification_work()
        }

        let small = work(128);
        let large = work(256);
        assert!(
            large <= small * 2 + 8,
            "normal import classification must remain linear: small={small} large={large}"
        );
    }

    #[test]
    fn missing_qualifier_lookup_work_scales_linearly() {
        fn work(count: usize, schema_fallback: bool) -> usize {
            let mut text = String::from("mod app\n");
            for index in 0..count {
                text.push_str(&format!("use dependency{index} as alias{index}\n"));
            }
            let source = veln_source::SourceFile::new("app.veln", text);
            let parsed = veln_syntax::parse(&source);
            assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
            let module = veln_ast::lower_surface_ast(&parsed.tree);
            let missing = vec!["missing".to_string()];

            reset_import_classification_work();
            let resolved = if schema_fallback {
                schema_composition_imported_use_for_path(&module, &missing, Some("app"))
            } else {
                normal_imported_use_for_path(&module, &missing, Some("app"))
            };
            assert!(resolved.is_none());
            import_classification_work()
        }

        for schema_fallback in [false, true] {
            let small = work(128, schema_fallback);
            let large = work(256, schema_fallback);
            eprintln!(
                "missing-qualifier import work: fallback={schema_fallback} 128={small} 256={large}"
            );
            assert!(
                large <= small * 2 + 8,
                "missing qualifier lookup must remain linear: fallback={schema_fallback} small={small} large={large}"
            );
        }
    }
}
