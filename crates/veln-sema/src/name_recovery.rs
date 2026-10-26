use std::collections::{BTreeMap, HashMap, HashSet};

use veln_ast::{NameClass, NameOccurrence, PublicAlias, SurfaceModule, UseDecl};

#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static IMPORT_CLASSIFICATION_WORK: Cell<usize> = const { Cell::new(0) };
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
}

#[cfg(test)]
pub(crate) fn import_classification_work() -> usize {
    IMPORT_CLASSIFICATION_WORK.get()
}

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

pub(crate) fn use_decl_has_invalid_module_segment(
    module: &SurfaceModule,
    use_decl: &UseDecl,
) -> bool {
    module.invalid_names.iter().any(|invalid| {
        invalid.class == NameClass::Module
            && (invalid.occurrence == NameOccurrence::PathSegment
                || (use_decl.alias_span.as_ref() == Some(&invalid.span)
                    && invalid.occurrence == NameOccurrence::Declaration))
            && invalid.span.file == use_decl.span.file
            && use_decl.span.start.offset <= invalid.span.start.offset
            && invalid.span.end.offset <= use_decl.span.end.offset
    })
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

pub(crate) fn use_decl_has_duplicate_target(module: &SurfaceModule, use_decl: &UseDecl) -> bool {
    if use_decl.recovered {
        return false;
    }
    module.uses.iter().any(|other| {
        record_import_classification_work();
        !std::ptr::eq(other, use_decl)
            && other.origin == veln_ast::UseOrigin::Source
            && use_decl.origin == veln_ast::UseOrigin::Source
            && !other.recovered
            && other.module_name == use_decl.module_name
            && other.written_name == use_decl.written_name
            && other.package == use_decl.package
    })
}

pub(crate) fn use_decl_has_reserved_alias(use_decl: &UseDecl) -> bool {
    use_decl.origin == veln_ast::UseOrigin::Source
        && use_decl.has_explicit_alias()
        && use_decl.alias == "prelude"
}

pub(crate) fn use_decl_has_source_alias_collision(
    module: &SurfaceModule,
    use_decl: &UseDecl,
) -> bool {
    if use_decl.recovered || use_decl_has_reserved_alias(use_decl) {
        return false;
    }
    let matches = module.uses.iter().filter(|other| {
        other.origin == veln_ast::UseOrigin::Source
            && !other.recovered
            && !use_decl_has_reserved_alias(other)
            && other.module_name == use_decl.module_name
            && other.alias == use_decl.alias
    });
    let mut count = 0usize;
    let mut has_explicit = false;
    for other in matches {
        count += 1;
        has_explicit |= other.has_explicit_alias();
    }
    count > 1 && has_explicit
}

pub(crate) fn use_decl_is_recovery_candidate(module: &SurfaceModule, use_decl: &UseDecl) -> bool {
    use_decl_has_invalid_module_segment(module, use_decl)
        && !use_decl.recovered
        && !use_decl_has_reserved_alias(use_decl)
        && !use_decl_has_duplicate_target(module, use_decl)
        && !use_decl_has_source_alias_collision(module, use_decl)
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
    duplicate_targets: HashMap<ImportTargetKey, Vec<usize>>,
    qualifiers: HashMap<ImportBindingKey, (usize, bool)>,
    exact_paths: HashMap<ImportBindingKey, Vec<usize>>,
}

struct NormalUseClassification {
    quarantined: Vec<bool>,
    colliding_qualifiers: HashSet<ImportBindingKey>,
    blocked_exact_paths: HashSet<ImportBindingKey>,
}

impl NormalUseGroups {
    fn collect(module: &SurfaceModule) -> Self {
        let invalid_module_spans = invalid_module_spans_by_file(module);
        let mut groups = Self {
            quarantined: vec![false; module.uses.len()],
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
        self.quarantined[index] = use_decl.recovered
            || reserved
            || use_decl_has_indexed_invalid_module_segment(invalid_module_spans, use_decl);
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

    fn finish(mut self) -> NormalUseClassification {
        let duplicate_targets = std::mem::take(&mut self.duplicate_targets);
        for indices in duplicate_targets.into_values() {
            self.quarantine_repeated_target(indices);
        }
        let blocked_exact_paths = blocked_exact_path_keys(self.exact_paths, &self.quarantined);
        NormalUseClassification {
            quarantined: self.quarantined,
            colliding_qualifiers: colliding_binding_keys(self.qualifiers),
            blocked_exact_paths,
        }
    }

    fn quarantine_repeated_target(&mut self, indices: Vec<usize>) {
        if indices.len() > 1 {
            for index in indices {
                self.quarantined[index] = true;
            }
        }
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

pub(crate) fn normal_use_decls(module: &SurfaceModule) -> Vec<UseDecl> {
    let classification = NormalUseGroups::collect(module).finish();
    module
        .uses
        .iter()
        .zip(classification.quarantined)
        .filter(|(_, quarantined)| !*quarantined)
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
        file_spans.sort_unstable();
    }
    spans
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
    let classification = NormalUseGroups::collect(module).finish();
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
    classification: NormalUseClassification,
    exact_paths: HashMap<ImportBindingKey, usize>,
    qualifiers: HashMap<ImportBindingKey, IndexedUseDecl>,
}

impl<'a> NormalImportSelectionIndex<'a> {
    pub(crate) fn new(module: &'a SurfaceModule) -> Self {
        record_normal_import_selection_work(|work| work.builds += 1);
        let classification = NormalUseGroups::collect(module).finish();
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
    classification: &NormalUseClassification,
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
    let classification = NormalUseGroups::collect(module).finish();
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
