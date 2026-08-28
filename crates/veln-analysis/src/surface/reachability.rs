use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};

use veln_ast::{
    BodyLine, BodyLineKind, CodecImplementationKind, Expr, ExprKind, Function, FunctionKind,
    Pattern, PatternKind, PublicAliasKind, SurfaceModule, UseDecl, Visibility,
};
use veln_project::classify_companion_source;
use veln_source::{SourceFile, SourceSpan};
use veln_syntax::{TokenKind, lex};

#[cfg(test)]
pub(crate) fn reachable_entry_module(
    module: &SurfaceModule,
    entry: &str,
    entry_kind: FunctionKind,
) -> SurfaceModule {
    reachable_entry_module_with_cache(module, entry, entry_kind, &ReachabilityCache::default())
}

#[derive(Default)]
pub(crate) struct ReachabilityCache {
    #[cfg(test)]
    function_targets: OnceCell<ReachabilityIndex>,
    separated_function_targets: OnceCell<ReachabilityIndex>,
    direct_callees: RefCell<HashMap<ReachableFunction, Vec<ReachableFunction>>>,
}

struct ReachabilityIndex {
    function_targets: FunctionTargetIndex,
    functions_by_name: HashMap<(FunctionKind, String), Vec<FunctionRef>>,
    functions_by_qualified_name: HashMap<(FunctionKind, String, String), Vec<FunctionRef>>,
}

impl ReachabilityIndex {
    fn new(inputs: &ReachabilityInputs<'_>, function_targets: Vec<FunctionTarget>) -> Self {
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

    fn function_refs(&self, key: &ReachableFunction) -> &[FunctionRef] {
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

#[derive(Clone, Copy)]
struct ReachabilityInputs<'a> {
    standard: Option<&'a SurfaceModule>,
    application: &'a SurfaceModule,
}

impl<'a> ReachabilityInputs<'a> {
    #[cfg(test)]
    fn combined(module: &'a SurfaceModule) -> Self {
        Self {
            standard: None,
            application: module,
        }
    }

    fn separated(standard: &'a SurfaceModule, application: &'a SurfaceModule) -> Self {
        Self {
            standard: Some(standard),
            application,
        }
    }

    fn module_header(&self) -> Option<veln_ast::ModuleHeader> {
        self.application
            .module
            .clone()
            .or_else(|| self.standard.and_then(|module| module.module.clone()))
    }

    fn cloned_declarations<T: Clone + 'a>(
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

    fn function_refs(&self) -> impl Iterator<Item = FunctionRef> + '_ {
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

    fn functions(&self) -> impl Iterator<Item = &'a Function> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.functions.iter())
            .chain(self.application.functions.iter())
    }

    fn function(&self, function_ref: FunctionRef) -> &'a Function {
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

    fn all_uses(&self) -> Vec<&'a UseDecl> {
        self.standard
            .into_iter()
            .flat_map(|module| module.uses.iter())
            .chain(self.application.uses.iter())
            .collect()
    }

    fn uses(&self) -> Vec<&'a UseDecl> {
        let invalid_names = self.invalid_names().collect::<Vec<_>>();
        self.all_uses()
            .into_iter()
            .filter(|use_decl| !use_decl_has_invalid_module_segment(use_decl, &invalid_names))
            .collect()
    }

    fn aliases(&self) -> impl Iterator<Item = &'a veln_ast::PublicAlias> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.aliases.iter())
            .chain(self.application.aliases.iter())
    }

    fn handlers(&self) -> Vec<&'a veln_ast::HandlerDecl> {
        self.standard
            .into_iter()
            .flat_map(|module| module.handlers.iter())
            .chain(self.application.handlers.iter())
            .collect()
    }

    fn types(&self) -> impl Iterator<Item = &'a veln_ast::TypeDecl> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.types.iter())
            .chain(self.application.types.iter())
    }

    fn invalid_names(&self) -> impl Iterator<Item = &'a veln_ast::InvalidName> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.invalid_names.iter())
            .chain(self.application.invalid_names.iter())
    }

    fn codecs(&self) -> impl Iterator<Item = &'a veln_ast::CodecDecl> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.codecs.iter())
            .chain(self.application.codecs.iter())
    }
}

#[derive(Clone, Copy)]
struct FunctionRef {
    input: ReachabilityInput,
    index: usize,
}

#[derive(Clone, Copy)]
enum ReachabilityInput {
    Standard,
    Application,
}

struct FunctionTargetIndex {
    all: Vec<FunctionTarget>,
    by_name: HashMap<String, Vec<usize>>,
    by_qualified_name: HashMap<(String, String), Vec<usize>>,
    by_shape: HashMap<FunctionShape, Vec<usize>>,
}

impl FunctionTargetIndex {
    fn new(all: Vec<FunctionTarget>) -> Self {
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

    fn named(&self, name: &str) -> impl Iterator<Item = &FunctionTarget> {
        self.by_name
            .get(name)
            .into_iter()
            .flatten()
            .map(|index| &self.all[*index])
    }

    fn qualified(&self, module_name: &str, name: &str) -> impl Iterator<Item = &FunctionTarget> {
        self.by_qualified_name
            .get(&(module_name.to_string(), name.to_string()))
            .into_iter()
            .flatten()
            .map(|index| &self.all[*index])
    }

    fn shaped(&self, shape: &FunctionShape) -> impl Iterator<Item = &FunctionTarget> {
        self.by_shape
            .get(shape)
            .into_iter()
            .flatten()
            .map(|index| &self.all[*index])
    }
}

#[cfg(test)]
pub(crate) mod reachability_counters {
    use std::cell::Cell;

    thread_local! {
        static FUNCTION_LOOKUP_SCANS: Cell<usize> = const { Cell::new(0) };
        static TARGET_RESOLUTION_SCANS: Cell<usize> = const { Cell::new(0) };
        static MATERIALIZED_FUNCTION_BODIES: Cell<usize> = const { Cell::new(0) };
        static RECOVERY_SELECTOR_CANDIDATE_SCANS: Cell<usize> = const { Cell::new(0) };
    }

    pub(crate) fn reset() {
        FUNCTION_LOOKUP_SCANS.set(0);
        TARGET_RESOLUTION_SCANS.set(0);
        MATERIALIZED_FUNCTION_BODIES.set(0);
        RECOVERY_SELECTOR_CANDIDATE_SCANS.set(0);
    }

    pub(crate) fn record_function_lookup_scan() {
        FUNCTION_LOOKUP_SCANS.set(FUNCTION_LOOKUP_SCANS.get() + 1);
    }

    pub(crate) fn record_target_resolution_scan() {
        TARGET_RESOLUTION_SCANS.set(TARGET_RESOLUTION_SCANS.get() + 1);
    }

    pub(crate) fn record_materialized_function_body() {
        MATERIALIZED_FUNCTION_BODIES.set(MATERIALIZED_FUNCTION_BODIES.get() + 1);
    }

    pub(crate) fn record_recovery_selector_candidate_scan() {
        RECOVERY_SELECTOR_CANDIDATE_SCANS.set(RECOVERY_SELECTOR_CANDIDATE_SCANS.get() + 1);
    }

    pub(crate) fn snapshot() -> (usize, usize, usize, usize) {
        (
            FUNCTION_LOOKUP_SCANS.get(),
            TARGET_RESOLUTION_SCANS.get(),
            MATERIALIZED_FUNCTION_BODIES.get(),
            RECOVERY_SELECTOR_CANDIDATE_SCANS.get(),
        )
    }
}

#[cfg(test)]
pub(crate) fn reachable_entry_module_with_cache(
    module: &SurfaceModule,
    entry: &str,
    entry_kind: FunctionKind,
    cache: &ReachabilityCache,
) -> SurfaceModule {
    let inputs = ReachabilityInputs::combined(module);
    let reachability_index = cache
        .function_targets
        .get_or_init(|| reachable_function_targets(&inputs));
    let companion_access_targets = companion_function_access_targets(&inputs);
    let reachable = reachable_functions(
        &inputs,
        entry,
        entry_kind,
        reachability_index,
        &companion_access_targets,
        cache,
    );
    module_with_reachable_functions(&inputs, &reachable)
}

pub(crate) fn reachable_entry_module_with_standard_cache(
    standard_module: &SurfaceModule,
    application_module: &SurfaceModule,
    entry: &str,
    entry_kind: FunctionKind,
    cache: &ReachabilityCache,
) -> SurfaceModule {
    let inputs = ReachabilityInputs::separated(standard_module, application_module);
    let reachability_index = cache
        .separated_function_targets
        .get_or_init(|| reachable_function_targets(&inputs));
    let companion_access_targets = companion_function_access_targets(&inputs);
    let reachable = reachable_functions(
        &inputs,
        entry,
        entry_kind,
        reachability_index,
        &companion_access_targets,
        cache,
    );
    module_with_reachable_functions(&inputs, &reachable)
}

fn reachable_function_targets(inputs: &ReachabilityInputs<'_>) -> ReachabilityIndex {
    let mut function_targets = function_targets(inputs);
    function_targets.extend(function_alias_targets(inputs, &function_targets));
    function_targets.extend(codec_with_targets(inputs));
    ReachabilityIndex::new(inputs, function_targets)
}

fn function_targets(inputs: &ReachabilityInputs<'_>) -> Vec<FunctionTarget> {
    inputs
        .functions()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(function_target)
        .collect()
}

fn function_target(function: &Function) -> Option<FunctionTarget> {
    let name = function.name.clone()?;
    let recovery = !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase);
    Some(FunctionTarget {
        name: name.clone(),
        module_name: function.module_name.clone(),
        target_name: name,
        target_module_name: function.module_name.clone(),
        target_node_id: function.node_id,
        visibility: function.visibility,
        shape: function_shape(function),
        bare_importable: true,
        requires_public_import: false,
        recovery,
    })
}

fn function_shape(function: &Function) -> FunctionShape {
    let mut fixed_arity = 0usize;
    let mut variadic = None;
    for param in &function.params {
        if param.is_variadic {
            variadic = param.ty.clone();
        } else {
            fixed_arity += 1;
        }
    }
    FunctionShape {
        fixed_arity,
        variadic,
    }
}

fn codec_with_targets(inputs: &ReachabilityInputs<'_>) -> Vec<FunctionTarget> {
    inputs
        .codecs()
        .flat_map(|codec| {
            let name = codec.name.clone()?;
            Some(
                codec
                    .implementations
                    .iter()
                    .filter_map(move |implementation| {
                        let CodecImplementationKind::With {
                            function: Some(function_name),
                        } = &implementation.kind
                        else {
                            return None;
                        };
                        let target = inputs.functions().find(|function| {
                            function.kind == FunctionKind::Function
                                && function.name.as_deref() == Some(function_name.as_str())
                                && function.module_name == codec.module_name
                        })?;
                        Some(FunctionTarget {
                            name: name.clone(),
                            module_name: codec.module_name.clone(),
                            target_name: function_name.clone(),
                            target_module_name: target.module_name.clone(),
                            target_node_id: target.node_id,
                            visibility: codec.visibility,
                            shape: function_shape(target),
                            bare_importable: false,
                            requires_public_import: true,
                            recovery: false,
                        })
                    }),
            )
        })
        .flatten()
        .collect()
}

fn reachable_functions(
    inputs: &ReachabilityInputs<'_>,
    entry: &str,
    entry_kind: FunctionKind,
    reachability_index: &ReachabilityIndex,
    companion_access_targets: &HashMap<String, String>,
    cache: &ReachabilityCache,
) -> HashSet<ReachableFunction> {
    let mut reachable = HashSet::<ReachableFunction>::new();
    let mut stack = vec![ReachableFunction {
        kind: entry_kind,
        name: entry.to_string(),
        module_name: None,
        node_id: None,
    }];

    while let Some(key) = stack.pop() {
        if !reachable.insert(key.clone()) {
            continue;
        }
        let cached_callees = cache.direct_callees.borrow().get(&key).cloned();
        let callees = cached_callees.unwrap_or_else(|| {
            let callees = reachability_index
                .function_refs(&key)
                .iter()
                .map(|function_ref| {
                    #[cfg(test)]
                    reachability_counters::record_function_lookup_scan();
                    inputs.function(*function_ref)
                })
                .filter(|function| {
                    key.node_id
                        .is_none_or(|node_id| function.node_id == node_id)
                })
                .flat_map(|function| {
                    direct_function_callees(
                        function,
                        inputs,
                        &reachability_index.function_targets,
                        companion_access_targets,
                    )
                })
                .collect::<Vec<_>>();
            cache
                .direct_callees
                .borrow_mut()
                .insert(key.clone(), callees.clone());
            callees
        });
        for callee in callees {
            if !reachable.contains(&callee) {
                stack.push(callee);
            }
        }
    }
    reachable
}

fn module_with_reachable_functions(
    inputs: &ReachabilityInputs<'_>,
    reachable: &HashSet<ReachableFunction>,
) -> SurfaceModule {
    let mut functions = materialize_reachable_functions(inputs, reachable);
    let reachable_invalid_name_spans = reachable_invalid_name_spans(inputs, &functions);
    functions.extend(materialize_quarantined_import_proof_functions(
        inputs, &functions,
    ));
    let invalid_names_by_declaration = inputs.cloned_declarations(|module| &module.invalid_names);
    let invalid_names = inputs
        .cloned_declarations(|module| &module.invalid_names)
        .into_iter()
        .filter(|invalid| invalid_name_is_reachable(invalid, &reachable_invalid_name_spans))
        .collect();
    SurfaceModule {
        module: inputs.module_header(),
        uses: inputs.cloned_declarations(|module| &module.uses),
        aliases: inputs
            .cloned_declarations(|module| &module.aliases)
            .into_iter()
            .filter(|alias| {
                !declaration_contains_invalid_name(&alias.span, &invalid_names_by_declaration)
                    || reachable_invalid_name_spans
                        .iter()
                        .any(|span| span.is_declaration(&alias.span))
            })
            .collect(),
        effects: inputs.cloned_declarations(|module| &module.effects),
        handlers: inputs
            .cloned_declarations(|module| &module.handlers)
            .into_iter()
            .filter(|handler| {
                reachable_invalid_name_spans
                    .iter()
                    .any(|span| span.is_declaration(&handler.span))
            })
            .collect(),
        types: inputs.cloned_declarations(|module| &module.types),
        schemas: inputs.cloned_declarations(|module| &module.schemas),
        codecs: inputs.cloned_declarations(|module| &module.codecs),
        functions,
        invalid_names,
    }
}

fn declaration_contains_invalid_name(
    declaration: &SourceSpan,
    invalid_names: &[veln_ast::InvalidName],
) -> bool {
    invalid_names
        .iter()
        .any(|invalid| span_contains(declaration, &invalid.span))
}

fn invalid_name_is_reachable(
    invalid: &veln_ast::InvalidName,
    reachable_spans: &[ReachableInvalidNameSpan],
) -> bool {
    if let Some(span) = &invalid.enclosing_function_span {
        return reachable_spans
            .iter()
            .any(|reachable| reachable.is_declaration(span));
    }
    reachable_spans.iter().any(|reachable| match reachable {
        ReachableInvalidNameSpan::Declaration(span) => span_contains(span, &invalid.span),
        ReachableInvalidNameSpan::Name(span) => span == &invalid.span,
    })
}

fn invalid_import_path_segment_spans(
    inputs: &ReachabilityInputs<'_>,
    invalid_names: &[&veln_ast::InvalidName],
) -> Vec<ReachableInvalidNameSpan> {
    inputs
        .all_uses()
        .into_iter()
        .filter(|use_decl| use_decl_has_invalid_module_segment(use_decl, invalid_names))
        .flat_map(|use_decl| {
            invalid_names
                .iter()
                .copied()
                .filter(move |invalid| invalid_module_segment_in_use_decl(use_decl, invalid))
                .map(|invalid| ReachableInvalidNameSpan::Name(invalid.span.clone()))
        })
        .collect()
}

fn use_decl_has_invalid_module_segment(
    use_decl: &UseDecl,
    invalid_names: &[&veln_ast::InvalidName],
) -> bool {
    invalid_names
        .iter()
        .copied()
        .any(|invalid| invalid_module_segment_in_use_decl(use_decl, invalid))
}

fn invalid_module_segment_in_use_decl(use_decl: &UseDecl, invalid: &veln_ast::InvalidName) -> bool {
    invalid.class == veln_ast::NameClass::Module
        && invalid.occurrence == veln_ast::NameOccurrence::PathSegment
        && span_contains(&use_decl.span, &invalid.span)
}

fn reachable_invalid_name_spans(
    inputs: &ReachabilityInputs<'_>,
    functions: &[Function],
) -> Vec<ReachableInvalidNameSpan> {
    let mut selector = ReachableInvalidNameSelector::new(inputs);
    let invalid_names = inputs.invalid_names().collect::<Vec<_>>();
    let mut spans = invalid_import_path_segment_spans(inputs, &invalid_names);
    spans.extend(
        functions
            .iter()
            .map(|function| function.span.clone())
            .map(ReachableInvalidNameSpan::Declaration),
    );
    for function in functions {
        selector.collect_function(function, &mut spans);
    }
    dedup_reachable_invalid_name_spans(&mut spans);
    spans
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ReachableInvalidNameSpan {
    Declaration(SourceSpan),
    Name(SourceSpan),
}

#[derive(Clone, Debug)]
struct ReachableRecoveryCandidate {
    spans: Vec<ReachableInvalidNameSpan>,
}

impl ReachableRecoveryCandidate {
    fn new(spans: Vec<ReachableInvalidNameSpan>) -> Self {
        Self { spans }
    }
}

impl ReachableInvalidNameSpan {
    fn is_declaration(&self, span: &SourceSpan) -> bool {
        matches!(self, Self::Declaration(reachable) if reachable == span)
    }
}

struct ReachableInvalidNameSelector<'a> {
    uses: Vec<&'a UseDecl>,
    handlers: Vec<&'a veln_ast::HandlerDecl>,
    functions_by_name: HashMap<(Option<String>, String), Vec<&'a Function>>,
    aliases_by_name: HashMap<(Option<String>, String), Vec<&'a veln_ast::PublicAlias>>,
    types_by_name: HashMap<(Option<String>, String), Vec<&'a veln_ast::TypeDecl>>,
    constructors_by_name: ConstructorVariantsByName<'a>,
    invalid_names: Vec<&'a veln_ast::InvalidName>,
    companion_access_targets: HashMap<String, String>,
}

type ConstructorVariantRef<'a> = (&'a veln_ast::TypeDecl, &'a veln_ast::TypeVariantDecl);
type ConstructorVariantsByName<'a> =
    HashMap<(Option<String>, String), Vec<ConstructorVariantRef<'a>>>;

fn index_functions_by_name<'a>(
    functions: &[&'a Function],
) -> HashMap<(Option<String>, String), Vec<&'a Function>> {
    let mut index = HashMap::<(Option<String>, String), Vec<&'a Function>>::new();
    for function in functions {
        if let Some(name) = &function.name {
            index
                .entry((function.module_name.clone(), name.clone()))
                .or_default()
                .push(*function);
        }
    }
    index
}

fn index_aliases_by_name<'a>(
    aliases: &[&'a veln_ast::PublicAlias],
) -> HashMap<(Option<String>, String), Vec<&'a veln_ast::PublicAlias>> {
    let mut index = HashMap::<(Option<String>, String), Vec<&'a veln_ast::PublicAlias>>::new();
    for alias in aliases {
        if let Some(name) = &alias.name {
            index
                .entry((alias.module_name.clone(), name.clone()))
                .or_default()
                .push(*alias);
        }
    }
    index
}

fn index_types_by_name<'a>(
    types: &[&'a veln_ast::TypeDecl],
) -> HashMap<(Option<String>, String), Vec<&'a veln_ast::TypeDecl>> {
    let mut index = HashMap::<(Option<String>, String), Vec<&'a veln_ast::TypeDecl>>::new();
    for type_decl in types {
        if let Some(name) = &type_decl.name {
            index
                .entry((type_decl.module_name.clone(), name.clone()))
                .or_default()
                .push(*type_decl);
        }
    }
    index
}

fn index_constructors_by_name<'a>(
    types: &[&'a veln_ast::TypeDecl],
) -> ConstructorVariantsByName<'a> {
    let mut index = ConstructorVariantsByName::new();
    for type_decl in types {
        for variant in &type_decl.variants {
            if let Some(name) = &variant.name {
                index
                    .entry((type_decl.module_name.clone(), name.clone()))
                    .or_default()
                    .push((*type_decl, variant));
            }
        }
    }
    index
}

impl<'a> ReachableInvalidNameSelector<'a> {
    fn new(inputs: &'a ReachabilityInputs<'_>) -> Self {
        let companion_access_targets = companion_function_access_targets(inputs);
        let aliases = inputs.aliases().collect::<Vec<_>>();
        let handlers = inputs.handlers();
        let types = inputs.types().collect::<Vec<_>>();
        let functions = inputs.functions().collect::<Vec<_>>();
        let functions_by_name = index_functions_by_name(&functions);
        let aliases_by_name = index_aliases_by_name(&aliases);
        let types_by_name = index_types_by_name(&types);
        let constructors_by_name = index_constructors_by_name(&types);
        Self {
            uses: inputs.uses(),
            handlers,
            functions_by_name,
            aliases_by_name,
            types_by_name,
            constructors_by_name,
            invalid_names: inputs.invalid_names().collect(),
            companion_access_targets,
        }
    }

    fn collect_function(&mut self, function: &Function, spans: &mut Vec<ReachableInvalidNameSpan>) {
        let mut local_bindings = function
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect::<Vec<_>>();
        for param in &function.params {
            self.collect_type_annotation(
                param.ty.as_deref(),
                function.module_name.as_deref(),
                spans,
            );
        }
        self.collect_type_annotation(
            function.return_type.as_deref(),
            function.module_name.as_deref(),
            spans,
        );
        for line in &function.body {
            match &line.kind {
                veln_ast::BodyLineKind::Let {
                    pattern,
                    annotation,
                    expr,
                } => {
                    self.collect_pattern(pattern, function.module_name.as_deref(), spans);
                    self.collect_type_annotation(
                        annotation.as_deref(),
                        function.module_name.as_deref(),
                        spans,
                    );
                    self.collect_expr(
                        expr,
                        function.module_name.as_deref(),
                        &local_bindings,
                        spans,
                    );
                    collect_pattern_binding_names(pattern, &mut local_bindings);
                }
                veln_ast::BodyLineKind::Expr { expr } => {
                    self.collect_expr(
                        expr,
                        function.module_name.as_deref(),
                        &local_bindings,
                        spans,
                    );
                }
            }
        }
    }

    fn collect_type_annotation(
        &mut self,
        annotation: Option<&str>,
        current_module: Option<&str>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        let Some(annotation) = annotation else {
            return;
        };
        let Ok(type_names) = veln_sema::type_annotation_reference_paths(annotation) else {
            return;
        };
        for path in type_names {
            self.select_type_name(&path, current_module, spans);
        }
    }

    fn collect_expr(
        &mut self,
        expr: &Expr,
        current_module: Option<&str>,
        local_bindings: &[String],
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        match &expr.kind {
            ExprKind::NamePath(segments) => {
                if !matches!(segments.as_slice(), [name] if local_bindings.iter().rev().any(|binding| binding == name))
                {
                    self.select_value_name(segments, current_module, spans);
                }
            }
            ExprKind::Hole { .. } => {}
            ExprKind::TypeApply { callee, type_args } => {
                self.collect_expr(callee, current_module, local_bindings, spans);
                for type_arg in type_args {
                    self.collect_type_annotation(Some(type_arg), current_module, spans);
                }
            }
            ExprKind::Call { callee, args } => {
                if let Some(segments) = callee_name_path(callee) {
                    if !matches!(segments.as_slice(), [name] if local_bindings.iter().rev().any(|binding| binding == name))
                    {
                        self.select_call_name(segments, current_module, args.len(), spans);
                    }
                } else {
                    self.collect_expr(callee, current_module, local_bindings, spans);
                }
                for arg in args {
                    self.collect_expr(arg, current_module, local_bindings, spans);
                }
            }
            ExprKind::Perform { args, .. } => {
                for arg in args {
                    self.collect_expr(arg, current_module, local_bindings, spans);
                }
            }
            ExprKind::Handle {
                body,
                handler,
                args,
                ..
            } => {
                self.select_handler(handler, current_module, spans);
                self.collect_expr(body, current_module, local_bindings, spans);
                for arg in args {
                    self.collect_expr(arg, current_module, local_bindings, spans);
                }
            }
            ExprKind::SchemaDecode {
                schema: _,
                input,
                base,
            } => {
                self.collect_expr(input, current_module, local_bindings, spans);
                self.collect_expr(base, current_module, local_bindings, spans);
            }
            ExprKind::SchemaEncode { schema: _, value } => {
                self.collect_expr(value, current_module, local_bindings, spans);
            }
            ExprKind::FieldAccess { base, .. }
            | ExprKind::Try(base)
            | ExprKind::Prefix { expr: base, .. } => {
                self.collect_expr(base, current_module, local_bindings, spans);
            }
            ExprKind::Record(fields) => {
                for field in fields {
                    self.collect_expr(&field.expr, current_module, local_bindings, spans);
                }
            }
            ExprKind::Dict(entries) => {
                for entry in entries {
                    self.collect_expr(&entry.key, current_module, local_bindings, spans);
                    self.collect_expr(&entry.value, current_module, local_bindings, spans);
                }
            }
            ExprKind::List(items) => {
                for item in items {
                    self.collect_expr(item, current_module, local_bindings, spans);
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.collect_expr(scrutinee, current_module, local_bindings, spans);
                for arm in arms {
                    self.collect_pattern(&arm.pattern, current_module, spans);
                    let mut arm_bindings = local_bindings.to_vec();
                    collect_pattern_binding_names(&arm.pattern, &mut arm_bindings);
                    self.collect_expr(&arm.expr, current_module, &arm_bindings, spans);
                }
            }
            ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => {
                self.collect_expr(condition, current_module, local_bindings, spans);
                self.collect_expr(then_branch, current_module, local_bindings, spans);
                for branch in else_if_branches {
                    self.collect_expr(&branch.condition, current_module, local_bindings, spans);
                    self.collect_expr(&branch.expr, current_module, local_bindings, spans);
                }
                self.collect_expr(else_branch, current_module, local_bindings, spans);
            }
            ExprKind::Binary { left, right, .. } => {
                self.collect_expr(left, current_module, local_bindings, spans);
                self.collect_expr(right, current_module, local_bindings, spans);
            }
            ExprKind::Missing
            | ExprKind::StringLiteral(_)
            | ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::BoolLiteral(_)
            | ExprKind::Unit => {}
        }
    }

    fn collect_pattern(
        &mut self,
        pattern: &Pattern,
        current_module: Option<&str>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        match &pattern.kind {
            PatternKind::Binding(_) => {}
            PatternKind::Constructor { name, args } => {
                self.select_constructor_name(name, current_module, None, spans);
                for arg in args {
                    self.collect_pattern(arg, current_module, spans);
                }
            }
            PatternKind::Record(fields) => {
                for field in fields {
                    self.collect_pattern(&field.pattern, current_module, spans);
                }
            }
            PatternKind::Wildcard
            | PatternKind::StringLiteral(_)
            | PatternKind::IntLiteral(_)
            | PatternKind::FloatLiteral(_)
            | PatternKind::BoolLiteral(_)
            | PatternKind::Unit => {}
        }
    }

    fn select_value_name(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        if self.has_valid_constructor(segments, current_module, None) {
            return;
        }
        if self.has_valid_function(segments, current_module, None) {
            return;
        }
        if self.has_valid_function_alias(segments, current_module) {
            return;
        }
        if same_module_recovery_path(segments) {
            self.select_unique_value_recovery(segments, current_module, spans);
        }
    }

    fn select_call_name(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: usize,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        if self.has_valid_function(segments, current_module, None)
            || self.has_valid_function_alias(segments, current_module)
            || self.has_valid_constructor(segments, current_module, None)
        {
            return;
        }
        if same_module_recovery_path(segments) {
            self.select_unique_call_recovery(segments, current_module, arg_count, spans);
        }
    }

    fn select_type_name(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        if self.has_valid_type(segments, current_module)
            || self.has_valid_type_alias(segments, current_module)
        {
            return;
        }
        if same_module_recovery_path(segments) {
            self.select_unique_type_recovery(segments, current_module, spans);
        }
    }

    fn select_constructor_name(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        if self.has_valid_constructor(segments, current_module, arg_count) {
            return;
        }
        if same_module_recovery_path(segments) {
            self.select_unique_constructor_recovery(segments, current_module, arg_count, spans);
        }
    }

    fn select_handler(
        &mut self,
        segments: &[String],
        current_module: Option<&str>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        if let Some(handler) = self.visible_handler(segments, current_module) {
            if spans.iter().any(|span| span.is_declaration(&handler.span)) {
                return;
            }
            spans.push(ReachableInvalidNameSpan::Declaration(handler.span.clone()));
            self.collect_handler(handler, spans);
        }
    }

    fn collect_handler(
        &mut self,
        handler: &veln_ast::HandlerDecl,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        let current_module = handler.module_name.as_deref();
        let mut local_bindings = handler
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect::<Vec<_>>();
        for param in &handler.params {
            self.collect_type_annotation(param.ty.as_deref(), current_module, spans);
        }
        for clause in &handler.operation_clauses {
            let binding_count = local_bindings.len();
            local_bindings.extend(clause.params.iter().map(|param| param.name.clone()));
            self.collect_expr(&clause.body, current_module, &local_bindings, spans);
            local_bindings.truncate(binding_count);
        }
    }

    fn has_valid_function(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
    ) -> bool {
        self.visible_functions(segments, current_module)
            .into_iter()
            .any(|function| {
                function
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_lowercase))
                    && arg_count
                        .is_none_or(|count| function_shape(function).accepts_arg_count(count))
            })
    }

    fn has_valid_function_alias(&self, segments: &[String], current_module: Option<&str>) -> bool {
        self.visible_aliases(segments, current_module, PublicAliasKind::Function)
            .into_iter()
            .any(|alias| {
                alias
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_lowercase))
            })
    }

    fn has_valid_type_alias(&self, segments: &[String], current_module: Option<&str>) -> bool {
        self.visible_aliases(segments, current_module, PublicAliasKind::Type)
            .into_iter()
            .any(|alias| {
                alias
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_uppercase))
            })
    }

    fn has_valid_type(&self, segments: &[String], current_module: Option<&str>) -> bool {
        self.visible_types(segments, current_module)
            .into_iter()
            .any(|type_decl| {
                type_decl
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_uppercase))
            })
    }

    fn has_valid_constructor(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
    ) -> bool {
        self.visible_constructor_variants(segments, current_module)
            .into_iter()
            .any(|(type_decl, variant)| {
                type_decl
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_uppercase))
                    && variant.name.as_ref().is_some_and(|name| {
                        name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
                    })
                    && arg_count.is_none_or(|count| variant.fields.len() == count)
            })
    }

    fn constructor_recovery_candidate(
        type_decl: &veln_ast::TypeDecl,
        variant: &veln_ast::TypeVariantDecl,
        arg_count: Option<usize>,
    ) -> bool {
        let invalid_type = type_decl
            .name
            .as_ref()
            .is_some_and(|name| !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase));
        let invalid_constructor = variant
            .name
            .as_ref()
            .is_some_and(|name| !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase));
        (invalid_type || invalid_constructor)
            && arg_count.is_none_or(|count| variant.fields.len() == count)
    }

    fn constructor_recovery_spans(
        &self,
        type_decl: &veln_ast::TypeDecl,
        variant: &veln_ast::TypeVariantDecl,
    ) -> Vec<ReachableInvalidNameSpan> {
        self.invalid_names
            .iter()
            .copied()
            .filter(|invalid| {
                (invalid.class == veln_ast::NameClass::Type
                    && span_contains(&type_decl.span, &invalid.span)
                    && type_decl.name.as_deref() == Some(invalid.name.as_str()))
                    || (invalid.class == veln_ast::NameClass::Constructor
                        && span_contains(&variant.span, &invalid.span)
                        && variant.name.as_deref() == Some(invalid.name.as_str()))
            })
            .map(|invalid| ReachableInvalidNameSpan::Name(invalid.span.clone()))
            .collect()
    }

    fn function_recovery_candidates(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
    ) -> Vec<ReachableRecoveryCandidate> {
        self.visible_functions(segments, current_module)
            .into_iter()
            .filter(|function| {
                function.name.as_ref().is_some_and(|name| {
                    !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                }) && arg_count
                    .is_none_or(|count| function_shape(function).accepts_arg_count(count))
            })
            .map(|function| {
                ReachableRecoveryCandidate::new(vec![ReachableInvalidNameSpan::Declaration(
                    function.span.clone(),
                )])
            })
            .chain(
                self.visible_aliases(segments, current_module, PublicAliasKind::Function)
                    .into_iter()
                    .filter(|alias| {
                        alias.name.as_ref().is_some_and(|name| {
                            !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                        })
                    })
                    .map(|alias| {
                        ReachableRecoveryCandidate::new(vec![
                            ReachableInvalidNameSpan::Declaration(alias.span.clone()),
                        ])
                    }),
            )
            .collect()
    }

    fn select_unique_type_recovery(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        let candidates = self
            .visible_types(segments, current_module)
            .into_iter()
            .filter(|type_decl| {
                type_decl.name.as_ref().is_some_and(|name| {
                    !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
                })
            })
            .map(|type_decl| type_decl.span.clone())
            .map(ReachableInvalidNameSpan::Declaration)
            .chain(
                self.visible_aliases(segments, current_module, PublicAliasKind::Type)
                    .into_iter()
                    .filter(|alias| {
                        alias.name.as_ref().is_some_and(|name| {
                            !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
                        })
                    })
                    .map(|alias| ReachableInvalidNameSpan::Declaration(alias.span.clone())),
            )
            .collect::<Vec<_>>();
        push_unique_reachable_invalid_name_span(candidates, spans);
    }

    fn select_unique_constructor_recovery(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        let candidates = self
            .visible_constructor_variants(segments, current_module)
            .into_iter()
            .filter(|(type_decl, variant)| {
                Self::constructor_recovery_candidate(type_decl, variant, arg_count)
            })
            .map(|(type_decl, variant)| {
                ReachableRecoveryCandidate::new(self.constructor_recovery_spans(type_decl, variant))
            })
            .collect::<Vec<_>>();
        push_unique_constructor_recovery_spans(candidates, spans);
    }

    fn constructor_recovery_candidates(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
    ) -> Vec<ReachableRecoveryCandidate> {
        self.visible_constructor_variants(segments, current_module)
            .into_iter()
            .filter(|(type_decl, variant)| {
                Self::constructor_recovery_candidate(type_decl, variant, arg_count)
            })
            .map(|(type_decl, variant)| {
                ReachableRecoveryCandidate::new(self.constructor_recovery_spans(type_decl, variant))
            })
            .collect()
    }

    fn select_unique_value_recovery(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        let mut candidates = self.constructor_recovery_candidates(segments, current_module, None);
        candidates.extend(self.function_recovery_candidates(segments, current_module, None));
        push_unique_constructor_recovery_spans(candidates, spans);
    }

    fn select_unique_call_recovery(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: usize,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        let mut candidates =
            self.function_recovery_candidates(segments, current_module, Some(arg_count));
        candidates.extend(self.constructor_recovery_candidates(
            segments,
            current_module,
            Some(arg_count),
        ));
        push_unique_constructor_recovery_spans(candidates, spans);
    }

    fn visible_functions(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Vec<&'a Function> {
        let target = visible_path_target(&self.uses, segments, current_module);
        let Some(leaf) = path_leaf(segments).map(str::to_string) else {
            return Vec::new();
        };
        self.functions_by_name
            .get(&(target.clone(), leaf))
            .into_iter()
            .flatten()
            .copied()
            .inspect(|_| {
                #[cfg(test)]
                reachability_counters::record_recovery_selector_candidate_scan();
            })
            .filter(move |function| {
                function.kind == FunctionKind::Function
                    && declaration_visible(
                        function.module_name.as_deref(),
                        function.visibility,
                        target.as_deref(),
                        current_module,
                    )
            })
            .collect()
    }

    fn visible_aliases(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        kind: PublicAliasKind,
    ) -> Vec<&'a veln_ast::PublicAlias> {
        let target = visible_path_target(&self.uses, segments, current_module);
        let Some(leaf) = path_leaf(segments).map(str::to_string) else {
            return Vec::new();
        };
        self.aliases_by_name
            .get(&(target.clone(), leaf))
            .into_iter()
            .flatten()
            .copied()
            .inspect(|_| {
                #[cfg(test)]
                reachability_counters::record_recovery_selector_candidate_scan();
            })
            .filter(move |alias| {
                alias.kind == kind
                    && declaration_visible(
                        alias.module_name.as_deref(),
                        Visibility::Public,
                        target.as_deref(),
                        current_module,
                    )
            })
            .collect()
    }

    fn visible_types(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Vec<&'a veln_ast::TypeDecl> {
        let target = visible_path_target(&self.uses, segments, current_module);
        let Some(leaf) = path_leaf(segments).map(str::to_string) else {
            return Vec::new();
        };
        self.types_by_name
            .get(&(target.clone(), leaf))
            .into_iter()
            .flatten()
            .copied()
            .inspect(|_| {
                #[cfg(test)]
                reachability_counters::record_recovery_selector_candidate_scan();
            })
            .filter(move |type_decl| {
                declaration_visible(
                    type_decl.module_name.as_deref(),
                    type_decl.visibility,
                    target.as_deref(),
                    current_module,
                )
            })
            .collect()
    }

    fn visible_constructor_variants(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Vec<(&'a veln_ast::TypeDecl, &'a veln_ast::TypeVariantDecl)> {
        let target = visible_path_target(&self.uses, segments, current_module);
        let Some(leaf) = path_leaf(segments).map(str::to_string) else {
            return Vec::new();
        };
        self.constructors_by_name
            .get(&(target.clone(), leaf))
            .into_iter()
            .flatten()
            .copied()
            .inspect(|_| {
                #[cfg(test)]
                reachability_counters::record_recovery_selector_candidate_scan();
            })
            .filter(move |(type_decl, _)| {
                declaration_visible(
                    type_decl.module_name.as_deref(),
                    type_decl.visibility,
                    target.as_deref(),
                    current_module,
                )
            })
            .collect()
    }

    fn visible_handler(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<&'a veln_ast::HandlerDecl> {
        let target = visible_path_target(&self.uses, segments, current_module);
        self.handlers.iter().copied().find(|handler| {
            handler.name.as_deref() == path_leaf(segments)
                && (declaration_visible(
                    handler.module_name.as_deref(),
                    handler.visibility,
                    target.as_deref(),
                    current_module,
                ) || companion_target_handler_visible(
                    handler,
                    target.as_deref(),
                    current_module,
                    &self.companion_access_targets,
                ))
        })
    }
}

fn companion_target_handler_visible(
    handler: &veln_ast::HandlerDecl,
    target_module: Option<&str>,
    current_module: Option<&str>,
    companion_access_targets: &HashMap<String, String>,
) -> bool {
    if handler.visibility == Visibility::Public || target_module != handler.module_name.as_deref() {
        return false;
    }
    current_module.is_some_and(|current_module| {
        handler.module_name.as_ref().is_some_and(|handler_module| {
            companion_access_targets
                .get(current_module)
                .is_some_and(|allowed_target| allowed_target == handler_module)
        })
    })
}

impl FunctionShape {
    fn accepts_arg_count(&self, arg_count: usize) -> bool {
        self.variadic.is_some() && arg_count >= self.fixed_arity
            || self.variadic.is_none() && arg_count == self.fixed_arity
    }
}

fn visible_path_target(
    uses: &[&UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<String> {
    match segments {
        [_] => current_module.map(str::to_string),
        [_, .., _] => imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
            .map(|use_decl| use_decl.name.clone()),
        _ => None,
    }
}

fn path_leaf(segments: &[String]) -> Option<&str> {
    segments.last().map(String::as_str)
}

fn same_module_recovery_path(segments: &[String]) -> bool {
    matches!(segments, [_])
}

fn declaration_visible(
    declaration_module: Option<&str>,
    visibility: Visibility,
    target_module: Option<&str>,
    current_module: Option<&str>,
) -> bool {
    match target_module {
        Some(target_module) if Some(target_module) != current_module => {
            declaration_module == Some(target_module) && visibility == Visibility::Public
        }
        Some(target_module) => declaration_module == Some(target_module),
        None => current_module.is_none() && declaration_module.is_none(),
    }
}

fn push_unique_reachable_invalid_name_span(
    mut candidates: Vec<ReachableInvalidNameSpan>,
    spans: &mut Vec<ReachableInvalidNameSpan>,
) {
    dedup_reachable_invalid_name_spans(&mut candidates);
    if let [span] = candidates.as_slice() {
        spans.push(span.clone());
    }
}

fn push_unique_constructor_recovery_spans(
    candidates: Vec<ReachableRecoveryCandidate>,
    spans: &mut Vec<ReachableInvalidNameSpan>,
) {
    if let [candidate] = candidates.as_slice() {
        let mut candidate_spans = candidate.spans.clone();
        dedup_reachable_invalid_name_spans(&mut candidate_spans);
        spans.extend(candidate_spans);
    }
}

fn dedup_reachable_invalid_name_spans(spans: &mut Vec<ReachableInvalidNameSpan>) {
    let mut seen = Vec::<ReachableInvalidNameSpan>::new();
    spans.retain(|span| {
        if seen.iter().any(|known| known == span) {
            false
        } else {
            seen.push(span.clone());
            true
        }
    });
}

fn collect_pattern_binding_names(pattern: &Pattern, bindings: &mut Vec<String>) {
    match &pattern.kind {
        PatternKind::Binding(name) => bindings.push(name.clone()),
        PatternKind::Constructor { args, .. } => {
            for arg in args {
                collect_pattern_binding_names(arg, bindings);
            }
        }
        PatternKind::Record(fields) => {
            for field in fields {
                collect_pattern_binding_names(&field.pattern, bindings);
            }
        }
        PatternKind::Wildcard
        | PatternKind::StringLiteral(_)
        | PatternKind::IntLiteral(_)
        | PatternKind::FloatLiteral(_)
        | PatternKind::BoolLiteral(_)
        | PatternKind::Unit => {}
    }
}

fn span_contains(container: &SourceSpan, span: &SourceSpan) -> bool {
    container.file == span.file
        && container.start.offset <= span.start.offset
        && span.end.offset <= container.end.offset
}

fn materialize_reachable_functions(
    inputs: &ReachabilityInputs<'_>,
    reachable: &HashSet<ReachableFunction>,
) -> Vec<Function> {
    inputs
        .functions()
        .filter(|function| {
            function.name.as_ref().is_some_and(|name| {
                reachable.contains(&ReachableFunction {
                    kind: function.kind,
                    name: name.clone(),
                    module_name: None,
                    node_id: None,
                }) || reachable.contains(&ReachableFunction {
                    kind: function.kind,
                    name: name.clone(),
                    module_name: function.module_name.clone(),
                    node_id: None,
                }) || reachable.contains(&ReachableFunction {
                    kind: function.kind,
                    name: name.clone(),
                    module_name: function.module_name.clone(),
                    node_id: Some(function.node_id),
                })
            })
        })
        .inspect(|_function| {
            #[cfg(test)]
            if !_function.body.is_empty() {
                reachability_counters::record_materialized_function_body();
            }
        })
        .cloned()
        .collect()
}

fn materialize_quarantined_import_proof_functions(
    inputs: &ReachabilityInputs<'_>,
    reachable_functions: &[Function],
) -> Vec<Function> {
    let invalid_names = inputs.invalid_names().collect::<Vec<_>>();
    let quarantined_modules = inputs
        .all_uses()
        .into_iter()
        .filter(|use_decl| use_decl_has_invalid_module_segment(use_decl, &invalid_names))
        .map(|use_decl| use_decl.name.as_str())
        .collect::<HashSet<_>>();
    if quarantined_modules.is_empty() {
        return Vec::new();
    }
    inputs
        .functions()
        .filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Public
                && function.module_name.as_deref().is_some_and(|module_name| {
                    quarantined_modules.contains(module_name)
                        && !reachable_functions.iter().any(|reachable| {
                            reachable.module_name.as_deref() == Some(module_name)
                                && reachable.name == function.name
                                && reachable.node_id == function.node_id
                        })
                })
        })
        .map(quarantined_import_proof_function)
        .collect()
}

fn quarantined_import_proof_function(function: &Function) -> Function {
    let mut proof = function.clone();
    proof.contracts.clear();
    proof.return_type = Some("()".to_string());
    for param in &mut proof.params {
        param.ty = Some("()".to_string());
    }
    proof.body = vec![BodyLine {
        node_id: function.node_id,
        kind: BodyLineKind::Expr {
            expr: Expr {
                node_id: function.node_id,
                kind: ExprKind::Unit,
                span: function.span.clone(),
            },
        },
        span: function.span.clone(),
    }];
    proof
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ReachableFunction {
    kind: FunctionKind,
    name: String,
    module_name: Option<String>,
    node_id: Option<veln_ast::NodeId>,
}

#[derive(Clone, Debug)]
struct FunctionTarget {
    name: String,
    module_name: Option<String>,
    target_name: String,
    target_module_name: Option<String>,
    target_node_id: veln_ast::NodeId,
    visibility: Visibility,
    shape: FunctionShape,
    bare_importable: bool,
    requires_public_import: bool,
    recovery: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct FunctionShape {
    fixed_arity: usize,
    variadic: Option<String>,
}

fn function_alias_targets(
    inputs: &ReachabilityInputs<'_>,
    function_targets: &[FunctionTarget],
) -> Vec<FunctionTarget> {
    let uses = inputs.uses();
    inputs
        .aliases()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
        .filter_map(|alias| {
            let name = alias.name.clone()?;
            let recovery = !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase);
            let target = target_for_alias_path(
                &alias.target,
                &uses,
                function_targets,
                alias.module_name.as_deref(),
            )?;
            if companion_alias_targets_imported_private_function(alias, target) {
                return None;
            }
            if target.recovery {
                return None;
            }
            Some(FunctionTarget {
                name,
                module_name: alias.module_name.clone(),
                target_name: target.target_name.clone(),
                target_module_name: target.target_module_name.clone(),
                target_node_id: target.target_node_id,
                visibility: Visibility::Public,
                shape: target.shape.clone(),
                bare_importable: true,
                requires_public_import: false,
                recovery,
            })
        })
        .collect()
}

fn companion_alias_targets_imported_private_function(
    alias: &veln_ast::PublicAlias,
    target: &FunctionTarget,
) -> bool {
    target.visibility != Visibility::Public
        && alias.module_name != target.target_module_name
        && classify_companion_source(alias.span.file.as_str()).is_some()
}

fn target_for_alias_path<'a>(
    segments: &[String],
    uses: &[&UseDecl],
    function_targets: &'a [FunctionTarget],
    current_module: Option<&str>,
) -> Option<&'a FunctionTarget> {
    match segments {
        [name] => function_targets.iter().find(|target| target.name == *name),
        [_, .., name] => {
            let use_decl =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
            let module_name = use_decl.name.as_str();
            function_targets.iter().find(|target| {
                target.name == *name
                    && target.module_name.as_deref() == Some(module_name)
                    && imported_target_is_visible(target, use_decl)
            })
        }
        _ => None,
    }
}

#[derive(Clone, Debug)]
struct LocalBinding {
    name: String,
    function_shape: Option<FunctionShape>,
}

struct FunctionCalleeContext<'a> {
    current_module: Option<&'a str>,
    uses: &'a [&'a UseDecl],
    function_targets: &'a FunctionTargetIndex,
    companion_access_targets: &'a HashMap<String, String>,
    handlers: &'a [&'a veln_ast::HandlerDecl],
    types: &'a [&'a veln_ast::TypeDecl],
}

fn direct_function_callees(
    function: &Function,
    inputs: &ReachabilityInputs<'_>,
    function_targets: &FunctionTargetIndex,
    companion_access_targets: &HashMap<String, String>,
) -> Vec<ReachableFunction> {
    let mut callees = Vec::new();
    let uses = inputs.uses();
    let handlers = inputs.handlers();
    let types = inputs.types().collect::<Vec<_>>();
    let context = FunctionCalleeContext {
        current_module: function.module_name.as_deref(),
        uses: &uses,
        function_targets,
        companion_access_targets,
        handlers: &handlers,
        types: &types,
    };
    let mut local_bindings = function
        .params
        .iter()
        .map(|param| LocalBinding {
            name: param.name.clone(),
            function_shape: param.ty.as_deref().and_then(function_type_shape),
        })
        .collect::<Vec<_>>();
    for contract in &function.contracts {
        collect_contract_callees(
            &contract.text,
            context.current_module,
            context.uses,
            function_targets,
            companion_access_targets,
            &mut callees,
        );
    }
    for line in &function.body {
        match &line.kind {
            veln_ast::BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                collect_function_callees(expr, &context, &local_bindings, &mut callees);
                collect_pattern_bindings(
                    pattern,
                    annotation.as_deref().and_then(function_type_shape),
                    &mut local_bindings,
                );
            }
            veln_ast::BodyLineKind::Expr { expr } => {
                collect_function_callees(expr, &context, &local_bindings, &mut callees);
            }
        }
    }
    callees
}

fn collect_contract_callees(
    predicate: &str,
    current_module: Option<&str>,
    uses: &[&UseDecl],
    function_targets: &FunctionTargetIndex,
    companion_access_targets: &HashMap<String, String>,
    callees: &mut Vec<ReachableFunction>,
) {
    let source = SourceFile::new("<contract>", predicate);
    let tokens = lex(&source)
        .tokens
        .into_iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
        .collect::<Vec<_>>();
    let mut index = 0usize;
    while index < tokens.len() {
        let name = &tokens[index];
        if name.kind != TokenKind::Ident {
            index += 1;
            continue;
        }
        let mut segments = vec![name.text.clone()];
        let mut next_index = index + 1;
        while next_index + 1 < tokens.len()
            && tokens[next_index].kind == TokenKind::DoubleColon
            && tokens[next_index + 1].kind == TokenKind::Ident
        {
            segments.push(tokens[next_index + 1].text.clone());
            next_index += 2;
        }
        let Some(next) = tokens.get(next_index) else {
            break;
        };
        if next.kind != TokenKind::LParen {
            index += 1;
            continue;
        }
        for callee in resolve_function_reference(
            &segments,
            current_module,
            uses,
            function_targets,
            companion_access_targets,
            None,
        ) {
            push_reachable(callees, callee);
        }
        index = next_index + 1;
    }
    collect_contract_function_value_references(
        &tokens,
        current_module,
        uses,
        function_targets,
        companion_access_targets,
        callees,
    );
}

fn collect_contract_function_value_references(
    tokens: &[veln_syntax::Token],
    current_module: Option<&str>,
    uses: &[&UseDecl],
    function_targets: &FunctionTargetIndex,
    _companion_access_targets: &HashMap<String, String>,
    callees: &mut Vec<ReachableFunction>,
) {
    let mut index = 0usize;
    while index < tokens.len() {
        if tokens[index].kind != TokenKind::Ident {
            index += 1;
            continue;
        }
        if index > 0
            && matches!(
                tokens[index - 1].kind,
                TokenKind::Dot | TokenKind::DoubleColon
            )
        {
            index += 1;
            continue;
        }
        if tokens
            .get(index + 1)
            .is_some_and(|token| matches!(token.kind, TokenKind::Dot | TokenKind::LParen))
        {
            index += 1;
            continue;
        }
        let segments = if tokens
            .get(index + 1)
            .is_some_and(|token| token.kind == TokenKind::DoubleColon)
            && tokens
                .get(index + 2)
                .is_some_and(|token| token.kind == TokenKind::Ident)
        {
            let mut segments = vec![tokens[index].text.clone()];
            index += 1;
            while tokens
                .get(index)
                .is_some_and(|token| token.kind == TokenKind::DoubleColon)
                && tokens
                    .get(index + 1)
                    .is_some_and(|token| token.kind == TokenKind::Ident)
            {
                segments.push(tokens[index + 1].text.clone());
                index += 2;
            }
            segments
        } else {
            let segments = vec![tokens[index].text.clone()];
            index += 1;
            segments
        };
        let public_or_same_module_access = HashMap::new();
        for callee in resolve_function_reference(
            &segments,
            current_module,
            uses,
            function_targets,
            &public_or_same_module_access,
            None,
        ) {
            push_reachable(callees, callee);
        }
    }
}

fn collect_function_callees(
    expr: &Expr,
    context: &FunctionCalleeContext<'_>,
    local_bindings: &[LocalBinding],
    callees: &mut Vec<ReachableFunction>,
) {
    let current_module = context.current_module;
    let uses = context.uses;
    let function_targets = context.function_targets;
    let companion_access_targets = context.companion_access_targets;
    let handlers = context.handlers;

    match &expr.kind {
        ExprKind::NamePath(segments) => {
            collect_function_name_reference(segments, context, local_bindings, None, callees);
        }
        ExprKind::TypeApply { callee, .. } => {
            collect_function_callees(callee, context, local_bindings, callees);
        }
        ExprKind::Call { callee, args } => {
            if let Some(segments) = callee_name_path(callee) {
                collect_function_name_reference(
                    segments,
                    context,
                    local_bindings,
                    Some(args.len()),
                    callees,
                );
            } else {
                collect_function_callees(callee, context, local_bindings, callees);
            }
            for arg in args {
                collect_function_callees(arg, context, local_bindings, callees);
            }
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_function_callees(arg, context, local_bindings, callees);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_handler_operation_clause_callees(
                expr,
                current_module,
                uses,
                function_targets,
                companion_access_targets,
                handlers,
                callees,
            );
            collect_function_callees(body, context, local_bindings, callees);
            for arg in args {
                collect_function_callees(arg, context, local_bindings, callees);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_function_callees(input, context, local_bindings, callees);
            collect_function_callees(base, context, local_bindings, callees);
        }
        ExprKind::SchemaEncode { value, .. } => {
            collect_function_callees(value, context, local_bindings, callees);
        }
        ExprKind::FieldAccess { base, .. } => {
            collect_function_callees(base, context, local_bindings, callees);
        }
        ExprKind::Try(inner) => collect_function_callees(inner, context, local_bindings, callees),
        ExprKind::Record(fields) => {
            for field in fields {
                collect_function_callees(&field.expr, context, local_bindings, callees);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_function_callees(&entry.key, context, local_bindings, callees);
                collect_function_callees(&entry.value, context, local_bindings, callees);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                collect_function_callees(item, context, local_bindings, callees);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_function_callees(scrutinee, context, local_bindings, callees);
            for arm in arms {
                let mut arm_bindings = local_bindings.to_vec();
                collect_pattern_bindings(&arm.pattern, None, &mut arm_bindings);
                collect_function_callees(&arm.expr, context, &arm_bindings, callees);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_function_callees(condition, context, local_bindings, callees);
            collect_function_callees(then_branch, context, local_bindings, callees);
            for branch in else_if_branches {
                collect_function_callees(&branch.condition, context, local_bindings, callees);
                collect_function_callees(&branch.expr, context, local_bindings, callees);
            }
            collect_function_callees(else_branch, context, local_bindings, callees);
        }
        ExprKind::Prefix { expr, .. } => {
            collect_function_callees(expr, context, local_bindings, callees);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_function_callees(left, context, local_bindings, callees);
            collect_function_callees(right, context, local_bindings, callees);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
    }
}

fn collect_pattern_bindings(
    pattern: &Pattern,
    function_shape: Option<FunctionShape>,
    bindings: &mut Vec<LocalBinding>,
) {
    match &pattern.kind {
        PatternKind::Binding(name) => bindings.push(LocalBinding {
            name: name.clone(),
            function_shape,
        }),
        PatternKind::Record(fields) => {
            for field in fields {
                collect_pattern_bindings(&field.pattern, None, bindings);
            }
        }
        PatternKind::Constructor { args, .. } => {
            for arg in args {
                collect_pattern_bindings(arg, None, bindings);
            }
        }
        PatternKind::Wildcard
        | PatternKind::StringLiteral(_)
        | PatternKind::IntLiteral(_)
        | PatternKind::FloatLiteral(_)
        | PatternKind::BoolLiteral(_)
        | PatternKind::Unit => {}
    }
}

fn callee_name_path(callee: &Expr) -> Option<&Vec<String>> {
    match &callee.kind {
        ExprKind::NamePath(segments) => Some(segments),
        ExprKind::TypeApply { callee, .. } => callee_name_path(callee),
        _ => None,
    }
}

fn collect_opaque_function_value_callees(
    shape: &FunctionShape,
    arg_count: Option<usize>,
    current_module: Option<&str>,
    uses: &[&UseDecl],
    function_targets: &FunctionTargetIndex,
    _companion_access_targets: &HashMap<String, String>,
    callees: &mut Vec<ReachableFunction>,
) {
    if current_module.is_some_and(|module| module.starts_with("std::")) {
        return;
    }
    if shape.variadic.is_some() && arg_count.is_some_and(|arg_count| arg_count < shape.fixed_arity)
    {
        return;
    }
    let public_or_same_module_access = HashMap::new();
    for target in function_targets.shaped(shape).filter(|target| {
        target_visible_from_current_module(
            target,
            current_module,
            uses,
            &public_or_same_module_access,
        )
    }) {
        push_reachable(
            callees,
            ReachableFunction {
                kind: FunctionKind::Function,
                name: target.name.clone(),
                module_name: target.module_name.clone(),
                node_id: None,
            },
        );
    }
}

fn target_visible_from_current_module(
    target: &FunctionTarget,
    current_module: Option<&str>,
    uses: &[&UseDecl],
    companion_access_targets: &HashMap<String, String>,
) -> bool {
    let target_module = target.module_name.as_deref();
    if current_module.is_none() || target_module == current_module {
        return true;
    }
    target_module.is_some_and(|module_name| {
        uses.iter().any(|use_decl| {
            use_decl.module_name.as_deref() == current_module
                && use_decl.origin == veln_ast::UseOrigin::Source
                && use_decl.name == module_name
                && imported_target_visible_from_module(
                    target,
                    use_decl,
                    current_module,
                    companion_access_targets,
                )
        })
    })
}

fn function_type_shape(annotation: &str) -> Option<FunctionShape> {
    let params = annotation.trim().strip_prefix("fn")?.trim_start();
    let params = params.strip_prefix('(')?;
    let mut depth = 0usize;
    let mut split_at = None;
    for (index, ch) in params.char_indices() {
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' if depth == 0 => {
                split_at = Some(index);
                break;
            }
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    let params = &params[..split_at?].trim();
    if params.is_empty() {
        return Some(FunctionShape {
            fixed_arity: 0,
            variadic: None,
        });
    }
    let mut parts = split_top_level_commas(params);
    let variadic = parts.last().and_then(|last| {
        last.strip_prefix("...")
            .map(str::trim)
            .filter(|element| !element.is_empty())
            .map(str::to_string)
    });
    if variadic.is_some() {
        parts.pop();
    }
    Some(FunctionShape {
        fixed_arity: parts.len(),
        variadic,
    })
}

fn split_top_level_commas(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());
    parts
}

fn path_has_valid_constructor(
    segments: &[String],
    arg_count: Option<usize>,
    current_module: Option<&str>,
    uses: &[&UseDecl],
    types: &[&veln_ast::TypeDecl],
) -> bool {
    let target = visible_path_target(uses, segments, current_module);
    let leaf = path_leaf(segments);
    types.iter().copied().any(|type_decl| {
        declaration_visible(
            type_decl.module_name.as_deref(),
            type_decl.visibility,
            target.as_deref(),
            current_module,
        ) && type_decl.variants.iter().any(|variant| {
            variant.name.as_deref() == leaf
                && variant
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_uppercase))
                && arg_count.is_none_or(|count| variant.fields.len() == count)
        })
    })
}

fn collect_function_name_reference(
    segments: &[String],
    context: &FunctionCalleeContext<'_>,
    local_bindings: &[LocalBinding],
    arg_count: Option<usize>,
    callees: &mut Vec<ReachableFunction>,
) {
    let current_module = context.current_module;
    let uses = context.uses;
    let function_targets = context.function_targets;
    let companion_access_targets = context.companion_access_targets;
    let types = context.types;

    if let [name] = segments
        && let Some(binding) = local_bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)
    {
        if let Some(shape) = &binding.function_shape {
            collect_opaque_function_value_callees(
                shape,
                arg_count,
                current_module,
                uses,
                function_targets,
                companion_access_targets,
                callees,
            );
        }
        return;
    }
    if path_has_valid_constructor(segments, None, current_module, uses, types) {
        return;
    }
    let public_or_same_module_access;
    let access_targets = if arg_count.is_some() {
        companion_access_targets
    } else {
        public_or_same_module_access = HashMap::new();
        &public_or_same_module_access
    };
    for callee in resolve_function_reference(
        segments,
        current_module,
        uses,
        function_targets,
        access_targets,
        arg_count,
    ) {
        push_reachable(callees, callee);
    }
}

fn collect_handler_operation_clause_callees(
    expr: &Expr,
    current_module: Option<&str>,
    uses: &[&UseDecl],
    function_targets: &FunctionTargetIndex,
    companion_access_targets: &HashMap<String, String>,
    handlers: &[&veln_ast::HandlerDecl],
    callees: &mut Vec<ReachableFunction>,
) {
    let ExprKind::Handle { handler, .. } = &expr.kind else {
        return;
    };
    let matching_handlers = handlers.iter().filter(|candidate| {
        let Some(name) = &candidate.name else {
            return false;
        };
        match handler.as_slice() {
            [segment] => name == segment && candidate.module_name.as_deref() == current_module,
            [_, .., segment] => {
                let Some(use_decl) =
                    imported_use_for_path(uses, &handler[..handler.len() - 1], current_module)
                else {
                    return false;
                };
                name == segment && candidate.module_name.as_deref() == Some(use_decl.name.as_str())
            }
            _ => false,
        }
    });
    for handler in matching_handlers {
        let context = FunctionCalleeContext {
            current_module,
            uses,
            function_targets,
            companion_access_targets,
            handlers,
            types: &[],
        };
        let mut local_bindings = handler
            .params
            .iter()
            .map(|param| LocalBinding {
                name: param.name.clone(),
                function_shape: param.ty.as_deref().and_then(function_type_shape),
            })
            .collect::<Vec<_>>();
        for clause in &handler.operation_clauses {
            let binding_count = local_bindings.len();
            local_bindings.extend(clause.params.iter().map(|param| LocalBinding {
                name: param.name.clone(),
                function_shape: None,
            }));
            collect_function_callees(&clause.body, &context, &local_bindings, callees);
            local_bindings.truncate(binding_count);
        }
    }
}

fn resolve_function_reference(
    segments: &[String],
    current_module: Option<&str>,
    uses: &[&UseDecl],
    function_targets: &FunctionTargetIndex,
    companion_access_targets: &HashMap<String, String>,
    arg_count: Option<usize>,
) -> Vec<ReachableFunction> {
    match segments {
        [name] => function_targets
            .named(name)
            .filter(|target| {
                #[cfg(test)]
                reachability_counters::record_target_resolution_scan();
                target.name == *name
                    && bare_target_visible(target, current_module, uses)
                    && recovery_target_accepts_arg_count(target, arg_count)
            })
            .map(|target| ReachableFunction {
                kind: FunctionKind::Function,
                name: target.target_name.clone(),
                module_name: target.target_module_name.clone(),
                node_id: Some(target.target_node_id),
            })
            .collect(),
        [_, .., name] => {
            let Some(use_decl) =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
            else {
                return Vec::new();
            };
            let module_name = use_decl.name.as_str();
            function_targets
                .qualified(module_name, name)
                .filter(|target| {
                    #[cfg(test)]
                    reachability_counters::record_target_resolution_scan();
                    imported_target_visible_from_module(
                        target,
                        use_decl,
                        current_module,
                        companion_access_targets,
                    ) && recovery_target_accepts_arg_count(target, arg_count)
                })
                .map(|target| ReachableFunction {
                    kind: FunctionKind::Function,
                    name: target.target_name.clone(),
                    module_name: target.target_module_name.clone(),
                    node_id: Some(target.target_node_id),
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

fn recovery_target_accepts_arg_count(target: &FunctionTarget, arg_count: Option<usize>) -> bool {
    !target.recovery || arg_count.is_none_or(|count| target.shape.accepts_arg_count(count))
}

fn imported_use_for_path<'a>(
    uses: &'a [&'a UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let module_path = segments.join("::");
    uses.iter().copied().find(|use_decl| {
        use_decl.module_name.as_deref() == current_module
            && (use_decl.name == module_path || use_decl.alias == module_path)
    })
}

fn imported_target_is_visible(target: &FunctionTarget, use_decl: &UseDecl) -> bool {
    if target.requires_public_import {
        return target.visibility == Visibility::Public;
    }
    use_decl.package.is_none() || target.visibility == Visibility::Public
}

fn imported_target_visible_from_module(
    target: &FunctionTarget,
    use_decl: &UseDecl,
    current_module: Option<&str>,
    companion_access_targets: &HashMap<String, String>,
) -> bool {
    if target.recovery {
        return false;
    }
    if target.visibility == Visibility::Public {
        return true;
    }
    if target.requires_public_import || use_decl.package.is_some() {
        return false;
    }
    if current_module.is_some_and(|module| module.starts_with("std::"))
        && target
            .module_name
            .as_deref()
            .is_some_and(|module| module.starts_with("std::"))
    {
        return true;
    }
    current_module.is_some_and(|current_module| {
        target.module_name.as_ref().is_some_and(|target_module| {
            companion_access_targets
                .get(current_module)
                .is_some_and(|allowed_target| allowed_target == target_module)
        })
    })
}

fn companion_function_access_targets(inputs: &ReachabilityInputs<'_>) -> HashMap<String, String> {
    inputs
        .functions()
        .filter_map(|function| {
            let companion = classify_companion_source(function.span.file.as_str())?;
            let companion_module = function.module_name.clone()?;
            let target_module = companion
                .target_path
                .strip_suffix(".veln")?
                .replace('/', "::");
            Some((companion_module, target_module))
        })
        .collect()
}

fn bare_target_visible(
    target: &FunctionTarget,
    current_module: Option<&str>,
    uses: &[&UseDecl],
) -> bool {
    let Some(current_module) = current_module else {
        return true;
    };
    if target.module_name.as_deref() == Some(current_module) {
        return true;
    }
    if target.recovery {
        return false;
    }
    target.bare_importable
        && target.module_name.as_deref().is_some_and(|module_name| {
            uses.iter().any(|use_decl| {
                use_decl.module_name.as_deref() == Some(current_module)
                    && use_decl.name == module_name
                    && imported_target_is_visible(target, use_decl)
            })
        })
}

fn push_reachable(callees: &mut Vec<ReachableFunction>, callee: ReachableFunction) {
    if !callees.iter().any(|known| known == &callee) {
        callees.push(callee);
    }
}
