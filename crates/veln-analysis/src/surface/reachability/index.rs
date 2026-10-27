use super::*;

#[derive(Default)]
pub(crate) struct ReachabilityCache {
    #[cfg(test)]
    pub(super) function_targets: OnceCell<ReachabilityIndex>,
    pub(super) separated_function_targets: OnceCell<ReachabilityIndex>,
    pub(super) direct_callees: RefCell<HashMap<ReachableFunction, Vec<ReachableFunction>>>,
}

pub(super) struct ReachabilityIndex {
    pub(super) function_targets: FunctionTargetIndex,
    functions_by_name: HashMap<(FunctionKind, String), Vec<FunctionRef>>,
    functions_by_qualified_name: HashMap<(FunctionKind, String, String), Vec<FunctionRef>>,
}

impl ReachabilityIndex {
    pub(super) fn new(
        inputs: &ReachabilityInputs<'_>,
        function_targets: Vec<FunctionTarget>,
    ) -> Self {
        let mut functions_by_name = HashMap::<(FunctionKind, String), Vec<FunctionRef>>::new();
        let mut functions_by_qualified_name =
            HashMap::<(FunctionKind, String, String), Vec<FunctionRef>>::new();
        for function_ref in inputs.function_refs() {
            let function = inputs.function(function_ref);
            let Some(name) = &function.name else {
                continue;
            };
            functions_by_name
                .entry((function.kind, name.clone()))
                .or_default()
                .push(function_ref);
            if let Some(module_name) = &function.module_name {
                functions_by_qualified_name
                    .entry((function.kind, module_name.clone(), name.clone()))
                    .or_default()
                    .push(function_ref);
            }
        }
        Self {
            function_targets: FunctionTargetIndex::new(function_targets),
            functions_by_name,
            functions_by_qualified_name,
        }
    }

    pub(super) fn function_refs(&self, key: &ReachableFunction) -> &[FunctionRef] {
        if let Some(module_name) = &key.module_name {
            self.functions_by_qualified_name
                .get(&(key.kind, module_name.clone(), key.name.clone()))
                .map(Vec::as_slice)
                .unwrap_or_default()
        } else {
            self.functions_by_name
                .get(&(key.kind, key.name.clone()))
                .map(Vec::as_slice)
                .unwrap_or_default()
        }
    }
}

pub(super) struct ReachabilityInputs<'a> {
    pub(super) standard: Option<&'a SurfaceModule>,
    pub(super) application: &'a SurfaceModule,
    use_index: UseIndex<'a>,
}

impl<'a> ReachabilityInputs<'a> {
    #[cfg(test)]
    pub(super) fn combined(module: &'a SurfaceModule) -> Self {
        Self {
            standard: None,
            application: module,
            use_index: UseIndex::new(None, module),
        }
    }

    pub(super) fn separated(standard: &'a SurfaceModule, application: &'a SurfaceModule) -> Self {
        Self {
            standard: Some(standard),
            application,
            use_index: UseIndex::new(Some(standard), application),
        }
    }

    pub(super) fn module_header(&self) -> Option<veln_ast::ModuleHeader> {
        self.application
            .module
            .clone()
            .or_else(|| self.standard.and_then(|module| module.module.clone()))
    }

    pub(super) fn cloned_declarations<T: Clone + 'a>(
        &self,
        select: impl Fn(&'a SurfaceModule) -> &'a [T],
    ) -> Vec<T> {
        self.standard
            .into_iter()
            .flat_map(|module| select(module).iter())
            .chain(select(self.application).iter())
            .cloned()
            .collect()
    }

    pub(super) fn function_refs(&self) -> impl Iterator<Item = FunctionRef> + '_ {
        let standard_len = self.standard.map_or(0, |module| module.functions.len());
        (0..standard_len)
            .map(|index| FunctionRef {
                input: ReachabilityInput::Standard,
                index,
            })
            .chain(
                (0..self.application.functions.len()).map(|index| FunctionRef {
                    input: ReachabilityInput::Application,
                    index,
                }),
            )
    }

    pub(super) fn functions(&self) -> impl Iterator<Item = &'a Function> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.functions.iter())
            .chain(self.application.functions.iter())
    }

    pub(super) fn function(&self, function_ref: FunctionRef) -> &'a Function {
        match function_ref.input {
            ReachabilityInput::Standard => {
                &self
                    .standard
                    .expect("standard function ref should have standard input")
                    .functions[function_ref.index]
            }
            ReachabilityInput::Application => &self.application.functions[function_ref.index],
        }
    }

    pub(super) fn uses(&self) -> Vec<&'a UseDecl> {
        self.use_index.valid.clone()
    }

    pub(super) fn import_selections(&self) -> &ImportSelectionIndex<'a> {
        &self.use_index.selections
    }

    pub(super) fn invalid_uses_with_segments(
        &self,
    ) -> impl Iterator<Item = (&'a UseDecl, &[&'a veln_ast::InvalidName])> + '_ {
        self.use_index
            .invalid
            .iter()
            .map(|invalid| (invalid.declaration, invalid.module_path_segments.as_slice()))
    }

    pub(super) fn aliases(&self) -> impl Iterator<Item = &'a veln_ast::PublicAlias> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.aliases.iter())
            .chain(self.application.aliases.iter())
    }

    pub(super) fn handlers(&self) -> Vec<&'a veln_ast::HandlerDecl> {
        self.standard
            .into_iter()
            .flat_map(|module| module.handlers.iter())
            .chain(self.application.handlers.iter())
            .collect()
    }

    pub(super) fn types(&self) -> impl Iterator<Item = &'a veln_ast::TypeDecl> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.types.iter())
            .chain(self.application.types.iter())
    }

    pub(super) fn invalid_names(&self) -> impl Iterator<Item = &'a veln_ast::InvalidName> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.invalid_names.iter())
            .chain(self.application.invalid_names.iter())
    }
}

struct UseIndex<'a> {
    valid: Vec<&'a UseDecl>,
    invalid: Vec<InvalidUse<'a>>,
    selections: ImportSelectionIndex<'a>,
}

struct InvalidUse<'a> {
    declaration: &'a UseDecl,
    module_path_segments: Vec<&'a veln_ast::InvalidName>,
}

type UseTarget = veln_ast::ResolvedImportTarget;
type InvalidNameSpans = HashSet<(veln_source::SourcePath, usize, usize)>;

impl<'a> UseIndex<'a> {
    fn new(standard: Option<&'a SurfaceModule>, application: &'a SurfaceModule) -> Self {
        let modules = standard
            .into_iter()
            .chain(std::iter::once(application))
            .collect::<Vec<_>>();
        let declarations = modules
            .iter()
            .flat_map(|module| module.uses.iter())
            .collect::<Vec<_>>();
        let exact_paths = declarations
            .iter()
            .map(|declaration| (use_decl_key(declaration), use_decl_exact_paths(declaration)))
            .collect::<HashMap<_, _>>();
        let blocked_aliases = blocked_import_aliases(&declarations);
        let duplicate_targets = duplicate_import_targets(&declarations);
        let (invalid_segments_by_file, invalid_alias_spans) = invalid_import_names(&modules);
        let blocked_exact_paths = blocked_exact_import_paths(
            &declarations,
            &exact_paths,
            &invalid_segments_by_file,
            &invalid_alias_spans,
            &duplicate_targets,
        );
        let (valid, invalid) = classify_use_declarations(
            &declarations,
            &invalid_segments_by_file,
            &invalid_alias_spans,
            &duplicate_targets,
            &blocked_aliases,
        );
        let blocked = BlockedImports {
            aliases: blocked_aliases,
            exact_paths: blocked_exact_paths,
        };
        let selections =
            ImportSelectionIndex::new(&declarations, &exact_paths, &valid, &invalid, &blocked);
        Self {
            valid,
            invalid,
            selections,
        }
    }
}

#[derive(Clone, Copy)]
enum ImportSelection<'a> {
    Unique(&'a UseDecl),
    Ambiguous,
    Blocked,
}

impl<'a> ImportSelection<'a> {
    fn add(slot: &mut HashMap<String, Self>, key: String, declaration: &'a UseDecl) {
        use std::collections::hash_map::Entry;

        match slot.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(Self::Unique(declaration));
            }
            Entry::Occupied(mut entry) => {
                entry.insert(Self::Ambiguous);
            }
        }
    }

    fn declaration(self) -> Option<&'a UseDecl> {
        match self {
            Self::Unique(declaration) => Some(declaration),
            Self::Ambiguous | Self::Blocked => None,
        }
    }
}

#[derive(Default)]
struct ModuleImportSelections<'a> {
    normal_exact: HashMap<String, ImportSelection<'a>>,
    normal_qualifier: HashMap<String, ImportSelection<'a>>,
    recovery_exact: HashMap<String, ImportSelection<'a>>,
    recovery_qualifier: HashMap<String, ImportSelection<'a>>,
}

pub(super) struct ImportSelectionIndex<'a> {
    root: ModuleImportSelections<'a>,
    modules: HashMap<String, ModuleImportSelections<'a>>,
}

impl<'a> ImportSelectionIndex<'a> {
    fn new(
        declarations: &[&'a UseDecl],
        exact_paths: &HashMap<usize, Vec<String>>,
        valid: &[&'a UseDecl],
        invalid: &[InvalidUse<'a>],
        blocked: &BlockedImports,
    ) -> Self {
        #[cfg(test)]
        reachability_counters::record_import_selection_index_build();

        let valid_ids = valid
            .iter()
            .map(|declaration| use_decl_key(declaration))
            .collect::<HashSet<_>>();
        let invalid_ids = invalid
            .iter()
            .map(|invalid| use_decl_key(invalid.declaration))
            .collect::<HashSet<_>>();
        let mut index = Self {
            root: ModuleImportSelections::default(),
            modules: HashMap::new(),
        };
        for declaration in declarations.iter().copied() {
            #[cfg(test)]
            reachability_counters::record_import_selection_source_visit();
            let declaration_key = use_decl_key(declaration);
            index.add_declaration(
                declaration,
                declaration_key,
                exact_paths,
                &valid_ids,
                &invalid_ids,
            );
        }
        index.block_exact_paths(&blocked.exact_paths);
        index.block_aliases(&blocked.aliases);
        index
    }

    fn add_declaration(
        &mut self,
        declaration: &'a UseDecl,
        declaration_key: usize,
        exact_paths: &HashMap<usize, Vec<String>>,
        valid_ids: &HashSet<usize>,
        invalid_ids: &HashSet<usize>,
    ) {
        let recovery = if valid_ids.contains(&declaration_key) {
            false
        } else if invalid_ids.contains(&declaration_key) {
            true
        } else {
            return;
        };
        let module = self.module_mut(declaration.module_name.as_deref());
        let (exact, qualifier) = if recovery {
            (&mut module.recovery_exact, &mut module.recovery_qualifier)
        } else {
            (&mut module.normal_exact, &mut module.normal_qualifier)
        };
        for path in exact_paths.get(&declaration_key).into_iter().flatten() {
            #[cfg(test)]
            reachability_counters::record_import_selection_exact_entry();
            ImportSelection::add(exact, path.clone(), declaration);
        }
        if simple_import_alias_matches(declaration, &declaration.alias) {
            #[cfg(test)]
            reachability_counters::record_import_selection_qualifier_entry();
            ImportSelection::add(qualifier, declaration.alias.clone(), declaration);
        }
    }

    fn block_exact_paths(&mut self, blocked: &BlockedExactImportPaths) {
        for (module_name, path) in blocked {
            let module = self.module_mut(module_name.as_deref());
            module
                .normal_exact
                .insert(path.clone(), ImportSelection::Blocked);
            module
                .recovery_exact
                .entry(path.clone())
                .or_insert(ImportSelection::Blocked);
        }
    }

    fn block_aliases(&mut self, blocked: &BlockedImportAliases) {
        for (module_name, alias) in blocked {
            let module = self.module_mut(module_name.as_deref());
            module
                .normal_qualifier
                .insert(alias.clone(), ImportSelection::Blocked);
            module
                .recovery_qualifier
                .insert(alias.clone(), ImportSelection::Blocked);
        }
    }

    fn module_mut(&mut self, module_name: Option<&str>) -> &mut ModuleImportSelections<'a> {
        match module_name {
            Some(module_name) => self.modules.entry(module_name.to_string()).or_default(),
            None => &mut self.root,
        }
    }

    fn module(&self, module_name: Option<&str>) -> Option<&ModuleImportSelections<'a>> {
        match module_name {
            Some(module_name) => self.modules.get(module_name),
            None => Some(&self.root),
        }
    }

    pub(super) fn select(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        recovery: bool,
    ) -> Option<&'a UseDecl> {
        #[cfg(test)]
        reachability_counters::record_import_selection_lookup(recovery);
        let path = segments.join("::");
        let module = self.module(current_module)?;
        let (exact, qualifier) = if recovery {
            (&module.recovery_exact, &module.recovery_qualifier)
        } else {
            (&module.normal_exact, &module.normal_qualifier)
        };
        #[cfg(test)]
        reachability_counters::record_import_selection_probe();
        if let Some(selection) = exact.get(path.as_str()) {
            return selection.declaration();
        }
        #[cfg(test)]
        reachability_counters::record_import_selection_probe();
        qualifier
            .get(path.as_str())
            .copied()
            .and_then(ImportSelection::declaration)
    }
}

fn blocked_exact_import_paths(
    declarations: &[&UseDecl],
    exact_paths: &HashMap<usize, Vec<String>>,
    invalid_segments_by_file: &HashMap<veln_source::SourcePath, Vec<&veln_ast::InvalidName>>,
    invalid_alias_spans: &InvalidNameSpans,
    duplicate_targets: &HashSet<UseTarget>,
) -> BlockedExactImportPaths {
    let mut groups = HashMap::<(Option<String>, String), (usize, bool)>::new();
    for declaration in declarations.iter().copied().filter(|declaration| {
        declaration.origin == veln_ast::UseOrigin::Source && !declaration.recovered
    }) {
        let module_path_is_valid = invalid_segments_by_file
            .get(&declaration.span.file)
            .is_none_or(|candidates| {
                invalid_segments_in_span(candidates, &declaration.span).is_empty()
            });
        let invalid_alias = declaration
            .alias_span
            .as_ref()
            .is_some_and(|span| invalid_alias_spans.contains(&span_key(span)));
        let duplicate_target = duplicate_targets.contains(&use_target(declaration));
        let quarantined = !is_valid_use(
            declaration,
            module_path_is_valid,
            invalid_alias,
            duplicate_target,
        );
        for path in exact_paths
            .get(&use_decl_key(declaration))
            .into_iter()
            .flatten()
            .cloned()
        {
            let group = groups
                .entry((declaration.module_name.clone(), path))
                .or_default();
            group.0 += 1;
            group.1 |= quarantined;
        }
    }
    groups
        .into_iter()
        .filter_map(|(path, (count, quarantined))| (count != 1 || quarantined).then_some(path))
        .collect()
}

fn blocked_import_aliases(declarations: &[&UseDecl]) -> BlockedImportAliases {
    let mut groups = HashMap::<(Option<String>, String), (usize, bool)>::new();
    for declaration in declarations.iter().copied().filter(|declaration| {
        declaration.origin == veln_ast::UseOrigin::Source
            && !declaration.recovered
            && !has_reserved_alias(declaration)
    }) {
        let group = groups
            .entry((declaration.module_name.clone(), declaration.alias.clone()))
            .or_default();
        group.0 += 1;
        group.1 |= declaration.has_explicit_alias();
    }
    groups
        .into_iter()
        .filter_map(|(alias, (count, has_explicit))| (count > 1 && has_explicit).then_some(alias))
        .collect()
}

fn duplicate_import_targets(declarations: &[&UseDecl]) -> HashSet<UseTarget> {
    let mut counts = HashMap::<UseTarget, usize>::new();
    for declaration in declarations
        .iter()
        .copied()
        .filter(|declaration| declaration.origin == veln_ast::UseOrigin::Source)
        .filter(|declaration| !declaration.recovered)
    {
        *counts.entry(use_target(declaration)).or_default() += 1;
    }
    counts
        .into_iter()
        .filter_map(|(target, count)| (count > 1).then_some(target))
        .collect()
}

fn invalid_import_names<'a>(
    modules: &[&'a SurfaceModule],
) -> (
    HashMap<veln_source::SourcePath, Vec<&'a veln_ast::InvalidName>>,
    InvalidNameSpans,
) {
    let mut segments_by_file = HashMap::<_, Vec<_>>::new();
    let mut alias_spans = HashSet::new();
    for invalid in modules.iter().flat_map(|module| &module.invalid_names) {
        if invalid.class != veln_ast::NameClass::Module {
            continue;
        }
        match invalid.occurrence {
            veln_ast::NameOccurrence::PathSegment => segments_by_file
                .entry(invalid.span.file.clone())
                .or_default()
                .push(invalid),
            veln_ast::NameOccurrence::Declaration => {
                alias_spans.insert(span_key(&invalid.span));
            }
            _ => {}
        }
    }
    for segments in segments_by_file.values_mut() {
        segments.sort_by_key(|invalid| invalid.span.start.offset);
    }
    (segments_by_file, alias_spans)
}

fn classify_use_declarations<'a>(
    declarations: &[&'a UseDecl],
    invalid_segments_by_file: &HashMap<veln_source::SourcePath, Vec<&'a veln_ast::InvalidName>>,
    invalid_alias_spans: &InvalidNameSpans,
    duplicate_targets: &HashSet<UseTarget>,
    blocked_aliases: &BlockedImportAliases,
) -> (Vec<&'a UseDecl>, Vec<InvalidUse<'a>>) {
    let mut valid = Vec::new();
    let mut invalid = Vec::new();
    for declaration in declarations.iter().copied() {
        let module_path_segments = invalid_segments_by_file
            .get(&declaration.span.file)
            .map(|candidates| invalid_segments_in_span(candidates, &declaration.span))
            .unwrap_or_default();
        let duplicate_target = declaration.origin == veln_ast::UseOrigin::Source
            && !declaration.recovered
            && duplicate_targets.contains(&use_target(declaration));
        let invalid_alias = declaration
            .alias_span
            .as_ref()
            .is_some_and(|span| invalid_alias_spans.contains(&span_key(span)));
        if is_valid_use(
            declaration,
            module_path_segments.is_empty(),
            invalid_alias,
            duplicate_target,
        ) {
            valid.push(declaration);
        } else if !module_path_segments.is_empty()
            && !blocked_aliases
                .contains(&(declaration.module_name.clone(), declaration.alias.clone()))
            && !duplicate_target
        {
            invalid.push(InvalidUse {
                declaration,
                module_path_segments,
            });
        }
    }
    (valid, invalid)
}

fn is_valid_use(
    declaration: &UseDecl,
    module_path_is_valid: bool,
    invalid_alias: bool,
    duplicate_target: bool,
) -> bool {
    module_path_is_valid
        && !invalid_alias
        && !declaration.recovered
        && !has_reserved_alias(declaration)
        && !duplicate_target
}

fn has_reserved_alias(declaration: &UseDecl) -> bool {
    declaration.has_explicit_alias() && declaration.alias == "prelude"
}

fn use_target(declaration: &UseDecl) -> UseTarget {
    declaration.resolved_target()
}

fn span_key(span: &SourceSpan) -> (veln_source::SourcePath, usize, usize) {
    (span.file.clone(), span.start.offset, span.end.offset)
}

fn invalid_segments_in_span<'a>(
    candidates: &[&'a veln_ast::InvalidName],
    span: &SourceSpan,
) -> Vec<&'a veln_ast::InvalidName> {
    let start = candidates.partition_point(|invalid| invalid.span.start.offset < span.start.offset);
    let end = candidates.partition_point(|invalid| invalid.span.start.offset <= span.end.offset);
    candidates[start..end]
        .iter()
        .copied()
        .filter(|invalid| {
            #[cfg(test)]
            reachability_counters::record_invalid_import_candidate_scan();
            span_contains(span, &invalid.span)
        })
        .collect()
}

#[derive(Clone, Copy)]
pub(super) struct FunctionRef {
    pub(super) input: ReachabilityInput,
    pub(super) index: usize,
}

#[derive(Clone, Copy)]
pub(super) enum ReachabilityInput {
    Standard,
    Application,
}

pub(super) struct FunctionTargetIndex {
    pub(super) all: Vec<FunctionTarget>,
    by_name: HashMap<String, Vec<usize>>,
    by_qualified_name: HashMap<(String, String), Vec<usize>>,
    by_shape: HashMap<FunctionShape, Vec<usize>>,
}

impl FunctionTargetIndex {
    pub(super) fn new(all: Vec<FunctionTarget>) -> Self {
        let mut by_name = HashMap::<String, Vec<usize>>::new();
        let mut by_qualified_name = HashMap::<(String, String), Vec<usize>>::new();
        let mut by_shape = HashMap::<FunctionShape, Vec<usize>>::new();
        for (index, target) in all.iter().enumerate() {
            by_name.entry(target.name.clone()).or_default().push(index);
            if let Some(module_name) = &target.module_name {
                by_qualified_name
                    .entry((module_name.clone(), target.name.clone()))
                    .or_default()
                    .push(index);
            }
            by_shape
                .entry(target.shape.clone())
                .or_default()
                .push(index);
        }
        Self {
            all,
            by_name,
            by_qualified_name,
            by_shape,
        }
    }

    pub(super) fn named(&self, name: &str) -> impl Iterator<Item = &FunctionTarget> {
        self.by_name
            .get(name)
            .into_iter()
            .flatten()
            .map(|index| &self.all[*index])
    }

    pub(super) fn qualified(
        &self,
        module_name: &str,
        name: &str,
    ) -> impl Iterator<Item = &FunctionTarget> {
        self.by_qualified_name
            .get(&(module_name.to_string(), name.to_string()))
            .into_iter()
            .flatten()
            .map(|index| &self.all[*index])
    }

    pub(super) fn shaped(&self, shape: &FunctionShape) -> impl Iterator<Item = &FunctionTarget> {
        self.by_shape
            .get(shape)
            .into_iter()
            .flatten()
            .map(|index| &self.all[*index])
    }
}
