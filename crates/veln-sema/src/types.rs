pub(crate) mod environment;
pub(crate) mod schema_types;
pub(crate) mod signatures;

pub(crate) use environment::*;
pub(crate) use schema_types::*;
pub(crate) use signatures::*;

use crate::schema::*;

#[cfg(test)]
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

use veln_ast::{
    BodyLine, BodyLineKind, CodecDecl, CodecDirection, CodecImplementationKind, DictEntry,
    EffectDecl, Expr, ExprKind, Function, FunctionKind, HandlerDecl, IfBranch, MatchArm, Pattern,
    PatternKind, PublicAlias, PublicAliasKind, RecordField, SchemaDecl, SchemaField, SurfaceModule,
    TypeDecl, UseDecl, Visibility, lower_surface_ast_with_module_identity,
};
use veln_project::classify_companion_source;
use veln_source::{SourceFile, SourceSpan, TextRange};

use crate::adt::{self, AdtRegistry};
use crate::effects::{
    concurrency_effects, is_stdio_call, prelude_effects, standard_library_effects,
};
use crate::semantic_model::{Binding, FunctionKey, Type};
use crate::type_syntax::{parse_type_annotation, parse_type_or_unknown};

#[derive(Clone)]
struct SchemaSymbolTable {
    schemas: Vec<SchemaSymbol>,
    aliases: Vec<SchemaAliasSymbol>,
}

#[derive(Clone)]
struct SchemaSymbol {
    name: String,
    module_name: Option<String>,
    visibility: Visibility,
    span: SourceSpan,
    unsupported_format_neutral_encode_field: Option<SchemaField>,
}

#[derive(Clone)]
struct SchemaAliasSymbol {
    name: String,
    module_name: Option<String>,
    target: Vec<String>,
}

struct ResolvedSchemaSymbol {
    name: String,
    module_name: Option<String>,
    span: SourceSpan,
    unsupported_format_neutral_encode_field: Option<SchemaField>,
}

struct SchemaAliasTarget {
    target: Vec<String>,
    module_name: Option<String>,
}

impl SchemaSymbolTable {
    fn extend(&mut self, other: Self) {
        self.schemas.extend(other.schemas);
        self.aliases.extend(other.aliases);
    }

    fn standard_subset(&self, module_names: &BTreeSet<String>) -> Self {
        Self {
            schemas: selected_standard_facts(&self.schemas, module_names, |symbol| {
                symbol.module_name.as_deref()
            }),
            aliases: selected_standard_facts(&self.aliases, module_names, |symbol| {
                symbol.module_name.as_deref()
            }),
        }
    }
}

#[derive(Clone)]
struct NamedSymbol {
    name: String,
    module_name: Option<String>,
    visibility: Visibility,
}

type FunctionAstMap<'a> = BTreeMap<FunctionKey, &'a Function>;
type FunctionSignatureMap = BTreeMap<FunctionKey, FunctionSignature>;
type FunctionReturnMap = BTreeMap<FunctionKey, Type>;
type PrivateSlotOmissions = (Vec<bool>, bool);
type PrivateSlotMap = BTreeMap<FunctionKey, PrivateSlotOmissions>;
type PrivateReferenceMap = BTreeMap<FunctionKey, BTreeSet<FunctionKey>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StandardSemanticIdentity {
    bundle_hash: u64,
    semantic_model: &'static str,
}

#[derive(Clone)]
pub struct ReusableStandardEnvironment {
    identity: StandardSemanticIdentity,
    module_names: BTreeSet<String>,
    declaration_counts: BTreeMap<StandardDeclarationKey, usize>,
    environment: Arc<TypeEnvironment>,
}

const STANDARD_SEMANTIC_MODEL: &str = "standard-semantic-signatures-v1";

impl ReusableStandardEnvironment {
    fn environment_for_modules(&self, module_names: &BTreeSet<String>) -> Arc<TypeEnvironment> {
        if module_names == &self.module_names {
            return self.environment.clone();
        }
        let selected_module_names = module_names
            .intersection(&self.module_names)
            .cloned()
            .collect::<BTreeSet<_>>();
        Arc::new(self.environment.standard_subset(&selected_module_names))
    }
}

#[cfg(test)]
impl ReusableStandardEnvironment {
    pub(crate) fn has_current_identity_for_test(&self) -> bool {
        self.identity == standard_semantic_identity()
    }

    pub(crate) fn with_current_identity_for_test(mut self) -> Self {
        self.identity = standard_semantic_identity();
        self
    }

    pub(crate) fn environment_for_modules_for_test(
        &self,
        module_names: &BTreeSet<String>,
    ) -> TypeEnvironment {
        self.environment_for_modules(module_names).as_ref().clone()
    }

    pub(crate) fn prepared_environment_count_for_test(&self) -> usize {
        1
    }

    pub(crate) fn selected_declaration_count_for_test(
        &self,
        module_names: &BTreeSet<String>,
    ) -> usize {
        self.declaration_counts
            .iter()
            .filter(|(key, _)| {
                key.module_name
                    .as_deref()
                    .is_some_and(|module_name| module_names.contains(module_name))
            })
            .map(|(_, count)| count)
            .sum()
    }
}

#[cfg(test)]
pub(crate) mod private_inference_counters {
    use super::*;

    thread_local! {
        static BODY_RETURN_SCANS: Cell<usize> = const { Cell::new(0) };
        static CALL_SITE_DISCOVERY_SCANS: Cell<usize> = const { Cell::new(0) };
        static CALL_SITE_SCANS: Cell<usize> = const { Cell::new(0) };
        static PRIVATE_REFERENCE_CANDIDATE_SCANS: Cell<usize> = const { Cell::new(0) };
        static PRIVATE_REFERENCE_INDEX_SCANS: Cell<usize> = const { Cell::new(0) };
        static PRELUDE_CALLBACK_DISCOVERY_SCANS: Cell<usize> = const { Cell::new(0) };
        static PRELUDE_CALLBACK_SCANS: Cell<usize> = const { Cell::new(0) };
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub(crate) struct Snapshot {
        pub(crate) body_return_scans: usize,
        pub(crate) call_site_discovery_scans: usize,
        pub(crate) call_site_scans: usize,
        pub(crate) private_reference_candidate_scans: usize,
        pub(crate) private_reference_index_scans: usize,
        pub(crate) prelude_callback_discovery_scans: usize,
        pub(crate) prelude_callback_scans: usize,
    }

    pub(crate) fn reset() {
        BODY_RETURN_SCANS.set(0);
        CALL_SITE_DISCOVERY_SCANS.set(0);
        CALL_SITE_SCANS.set(0);
        PRIVATE_REFERENCE_CANDIDATE_SCANS.set(0);
        PRIVATE_REFERENCE_INDEX_SCANS.set(0);
        PRELUDE_CALLBACK_DISCOVERY_SCANS.set(0);
        PRELUDE_CALLBACK_SCANS.set(0);
    }

    pub(crate) fn snapshot() -> Snapshot {
        Snapshot {
            body_return_scans: BODY_RETURN_SCANS.get(),
            call_site_discovery_scans: CALL_SITE_DISCOVERY_SCANS.get(),
            call_site_scans: CALL_SITE_SCANS.get(),
            private_reference_candidate_scans: PRIVATE_REFERENCE_CANDIDATE_SCANS.get(),
            private_reference_index_scans: PRIVATE_REFERENCE_INDEX_SCANS.get(),
            prelude_callback_discovery_scans: PRELUDE_CALLBACK_DISCOVERY_SCANS.get(),
            prelude_callback_scans: PRELUDE_CALLBACK_SCANS.get(),
        }
    }

    pub(super) fn record_body_return_scan() {
        BODY_RETURN_SCANS.set(BODY_RETURN_SCANS.get() + 1);
    }

    pub(super) fn record_call_site_discovery_scan() {
        CALL_SITE_DISCOVERY_SCANS.set(CALL_SITE_DISCOVERY_SCANS.get() + 1);
    }

    pub(super) fn record_call_site_scan() {
        CALL_SITE_SCANS.set(CALL_SITE_SCANS.get() + 1);
    }

    pub(super) fn record_private_reference_candidate_scan() {
        PRIVATE_REFERENCE_CANDIDATE_SCANS.set(PRIVATE_REFERENCE_CANDIDATE_SCANS.get() + 1);
    }

    pub(super) fn record_private_reference_index_scan() {
        PRIVATE_REFERENCE_INDEX_SCANS.set(PRIVATE_REFERENCE_INDEX_SCANS.get() + 1);
    }

    pub(super) fn record_prelude_callback_discovery_scan() {
        PRELUDE_CALLBACK_DISCOVERY_SCANS.set(PRELUDE_CALLBACK_DISCOVERY_SCANS.get() + 1);
    }

    pub(super) fn record_prelude_callback_scan() {
        PRELUDE_CALLBACK_SCANS.set(PRELUDE_CALLBACK_SCANS.get() + 1);
    }
}

#[cfg(test)]
pub(crate) mod effect_inference_counters {
    use super::*;

    thread_local! {
        static DEPENDENCY_DISCOVERY_SCANS: Cell<usize> = const { Cell::new(0) };
        static FUNCTION_BODY_COLLECTIONS: Cell<usize> = const { Cell::new(0) };
        static HANDLER_OPERATION_CLAUSE_EVALUATIONS: Cell<usize> = const { Cell::new(0) };
        static CHANGED_REEVALUATIONS: Cell<usize> = const { Cell::new(0) };
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub(crate) struct Snapshot {
        pub(crate) dependency_discovery_scans: usize,
        pub(crate) function_body_collections: usize,
        pub(crate) handler_operation_clause_evaluations: usize,
        pub(crate) changed_reevaluations: usize,
    }

    pub(crate) fn reset() {
        DEPENDENCY_DISCOVERY_SCANS.set(0);
        FUNCTION_BODY_COLLECTIONS.set(0);
        HANDLER_OPERATION_CLAUSE_EVALUATIONS.set(0);
        CHANGED_REEVALUATIONS.set(0);
    }

    pub(crate) fn snapshot() -> Snapshot {
        Snapshot {
            dependency_discovery_scans: DEPENDENCY_DISCOVERY_SCANS.get(),
            function_body_collections: FUNCTION_BODY_COLLECTIONS.get(),
            handler_operation_clause_evaluations: HANDLER_OPERATION_CLAUSE_EVALUATIONS.get(),
            changed_reevaluations: CHANGED_REEVALUATIONS.get(),
        }
    }

    pub(super) fn record_dependency_discovery_scan() {
        DEPENDENCY_DISCOVERY_SCANS.set(DEPENDENCY_DISCOVERY_SCANS.get() + 1);
    }

    pub(super) fn record_function_body_collection() {
        FUNCTION_BODY_COLLECTIONS.set(FUNCTION_BODY_COLLECTIONS.get() + 1);
    }

    pub(super) fn record_handler_operation_clause_evaluation() {
        HANDLER_OPERATION_CLAUSE_EVALUATIONS.set(HANDLER_OPERATION_CLAUSE_EVALUATIONS.get() + 1);
    }

    pub(super) fn record_changed_reevaluation() {
        CHANGED_REEVALUATIONS.set(CHANGED_REEVALUATIONS.get() + 1);
    }
}

#[cfg(test)]
pub mod standard_reuse_counters {
    use super::*;

    static STANDARD_PREPARES: AtomicUsize = AtomicUsize::new(0);
    static STANDARD_ENVIRONMENT_BUILDS: AtomicUsize = AtomicUsize::new(0);
    static APPLICATION_PREPARES: AtomicUsize = AtomicUsize::new(0);

    pub fn reset() {
        STANDARD_PREPARES.store(0, Ordering::SeqCst);
        STANDARD_ENVIRONMENT_BUILDS.store(0, Ordering::SeqCst);
        APPLICATION_PREPARES.store(0, Ordering::SeqCst);
    }

    pub fn standard_prepares() -> usize {
        STANDARD_PREPARES.load(Ordering::SeqCst)
    }

    pub fn standard_environment_builds() -> usize {
        STANDARD_ENVIRONMENT_BUILDS.load(Ordering::SeqCst)
    }

    pub fn application_prepares() -> usize {
        APPLICATION_PREPARES.load(Ordering::SeqCst)
    }

    pub(super) fn record_standard_prepare() {
        STANDARD_PREPARES.fetch_add(1, Ordering::SeqCst);
    }

    pub(super) fn record_standard_environment_build() {
        STANDARD_ENVIRONMENT_BUILDS.fetch_add(1, Ordering::SeqCst);
    }

    pub(super) fn record_application_prepare() {
        APPLICATION_PREPARES.fetch_add(1, Ordering::SeqCst);
    }
}

pub fn prepare_reusable_standard_environment(
    module: &SurfaceModule,
) -> ReusableStandardEnvironment {
    #[cfg(test)]
    standard_reuse_counters::record_standard_prepare();
    let standard_module = module_without_application_declarations(module);
    let module_names = module_standard_names(&standard_module);
    #[cfg(test)]
    standard_reuse_counters::record_standard_environment_build();
    let environment = TypeEnvironment::from_module(&standard_module);
    ReusableStandardEnvironment {
        identity: prepared_standard_semantic_identity(&standard_module, &module_names),
        module_names,
        declaration_counts: standard_declaration_counts(&standard_module),
        environment: Arc::new(environment),
    }
}

pub fn prepare_current_reusable_standard_environment(
    module: &SurfaceModule,
) -> ReusableStandardEnvironment {
    let mut environment = prepare_reusable_standard_environment(module);
    environment.identity = standard_semantic_identity();
    environment
}

pub fn standard_semantic_identity() -> StandardSemanticIdentity {
    let bundle = veln_stdlib::package_bundle();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    STANDARD_SEMANTIC_MODEL.hash(&mut hasher);
    bundle.manifest.hash(&mut hasher);
    bundle.exports.hash(&mut hasher);
    for file in bundle.files {
        file.path.hash(&mut hasher);
        file.text.hash(&mut hasher);
    }
    StandardSemanticIdentity {
        bundle_hash: hasher.finish(),
        semantic_model: STANDARD_SEMANTIC_MODEL,
    }
}

fn application_module_is_empty(module: &SurfaceModule) -> bool {
    module.uses.is_empty()
        && module.aliases.is_empty()
        && module.effects.is_empty()
        && module.handlers.is_empty()
        && module.types.is_empty()
        && module.schemas.is_empty()
        && module.codecs.is_empty()
        && module.functions.is_empty()
}

fn standard_fact_in_selected_module(
    module_name: Option<&str>,
    selected_modules: &BTreeSet<String>,
) -> bool {
    module_name.is_none_or(|module_name| selected_modules.contains(module_name))
}

fn selected_standard_facts<T: Clone>(
    facts: &[T],
    selected_modules: &BTreeSet<String>,
    module_name: impl for<'a> Fn(&'a T) -> Option<&'a str>,
) -> Vec<T> {
    facts
        .iter()
        .filter(|fact| standard_fact_in_selected_module(module_name(fact), selected_modules))
        .cloned()
        .collect()
}

fn selected_standard_access_targets(
    access_targets: &BTreeMap<String, String>,
    selected_modules: &BTreeSet<String>,
) -> BTreeMap<String, String> {
    access_targets
        .iter()
        .filter(|(module, target)| {
            selected_modules.contains(module.as_str()) && selected_modules.contains(target.as_str())
        })
        .map(|(module, target)| (module.clone(), target.clone()))
        .collect()
}

fn module_without_reusable_standard_declarations(
    module: &SurfaceModule,
    standard: &ReusableStandardEnvironment,
) -> SurfaceModule {
    let mut remaining_standard_declarations = standard.declaration_counts.clone();
    filter_module_declarations(module, |decl| {
        if !decl
            .module_name()
            .is_some_and(|module_name| standard.module_names.contains(module_name))
        {
            return true;
        }
        let key = standard_declaration_key(decl);
        let Some(count) = remaining_standard_declarations.get_mut(&key) else {
            return true;
        };
        *count -= 1;
        if *count == 0 {
            remaining_standard_declarations.remove(&key);
        }
        false
    })
}

fn reusable_standard_module_names_for(module: &SurfaceModule) -> BTreeSet<String> {
    module_standard_names(&module_without_application_declarations(module))
}

fn module_without_application_declarations(module: &SurfaceModule) -> SurfaceModule {
    filter_module_declarations(module, is_embedded_standard_declaration)
}

fn module_standard_names(module: &SurfaceModule) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    collect_standard_names(&module.uses, &mut names);
    collect_standard_names(&module.aliases, &mut names);
    collect_standard_names(&module.effects, &mut names);
    collect_standard_names(&module.handlers, &mut names);
    collect_standard_names(&module.types, &mut names);
    collect_standard_names(&module.schemas, &mut names);
    collect_standard_names(&module.codecs, &mut names);
    collect_standard_names(&module.functions, &mut names);
    names
}

fn standard_declaration_counts(module: &SurfaceModule) -> BTreeMap<StandardDeclarationKey, usize> {
    let mut counts = BTreeMap::new();
    count_standard_declarations(&module.uses, &mut counts);
    count_standard_declarations(&module.aliases, &mut counts);
    count_standard_declarations(&module.effects, &mut counts);
    count_standard_declarations(&module.handlers, &mut counts);
    count_standard_declarations(&module.types, &mut counts);
    count_standard_declarations(&module.schemas, &mut counts);
    count_standard_declarations(&module.codecs, &mut counts);
    count_standard_declarations(&module.functions, &mut counts);
    counts
}

fn prepared_standard_semantic_identity(
    module: &SurfaceModule,
    module_names: &BTreeSet<String>,
) -> StandardSemanticIdentity {
    if module_names == &embedded_standard_module_names()
        && semantic_module_fingerprint(module) == embedded_standard_module_fingerprint()
    {
        return standard_semantic_identity();
    }

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    STANDARD_SEMANTIC_MODEL.hash(&mut hasher);
    module_names.hash(&mut hasher);
    StandardSemanticIdentity {
        bundle_hash: hasher.finish(),
        semantic_model: STANDARD_SEMANTIC_MODEL,
    }
}

fn embedded_standard_module_names() -> BTreeSet<String> {
    veln_stdlib::package_bundle()
        .files
        .iter()
        .filter_map(|file| standard_module_name_from_bundle_path(file.path))
        .collect()
}

fn standard_module_name_from_bundle_path(path: &str) -> Option<String> {
    path.strip_suffix(".veln")
        .map(|module| format!("std::{}", module.replace('/', "::")))
}

fn embedded_standard_module_fingerprint() -> u64 {
    semantic_module_fingerprint(&embedded_standard_surface_module())
}

fn semantic_module_fingerprint(module: &SurfaceModule) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!("{module:?}").hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn embedded_standard_surface_module() -> SurfaceModule {
    let mut merged = SurfaceModule {
        module: None,
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        functions: Vec::new(),
    };
    let mut modules = veln_stdlib::package_bundle()
        .files
        .iter()
        .filter_map(|file| {
            standard_module_name_from_bundle_path(file.path).map(|name| (name, file))
        })
        .collect::<Vec<_>>();
    modules.sort_by(|left, right| left.0.cmp(&right.0));
    for (module_name, file) in modules {
        let source = SourceFile::new(file.path, file.text);
        let parsed = veln_syntax::parse(&source);
        if !parsed.diagnostics.is_empty() {
            continue;
        }
        let mut module = lower_surface_ast_with_module_identity(
            &parsed.tree,
            module_name,
            source.span(TextRange::new(0, 0)),
        );
        rewrite_standard_bundle_import_targets(&mut module.uses);
        merge_standard_surface_module(&mut merged, module);
    }
    merged
}

fn rewrite_standard_bundle_import_targets(uses: &mut [UseDecl]) {
    for use_decl in uses {
        if use_decl.package.is_some() || use_decl.name.starts_with("std::") {
            continue;
        }
        use_decl.name = format!("std::{}", use_decl.name);
    }
}

fn merge_standard_surface_module(merged: &mut SurfaceModule, module: SurfaceModule) {
    if merged.module.is_none() {
        merged.module = module.module;
    }
    merged.uses.extend(module.uses);
    merged.aliases.extend(module.aliases);
    merged.effects.extend(module.effects);
    merged.handlers.extend(module.handlers);
    merged.types.extend(module.types);
    merged.schemas.extend(module.schemas);
    merged.codecs.extend(module.codecs);
    merged.functions.extend(module.functions);
}

fn collect_standard_names<T: StandardDeclaration>(decls: &[T], names: &mut BTreeSet<String>) {
    names.extend(
        decls
            .iter()
            .filter_map(StandardDeclaration::module_name)
            .filter(|name| is_standard_module_name(Some(name)))
            .map(str::to_string),
    );
}

fn count_standard_declarations<T: StandardDeclaration>(
    decls: &[T],
    counts: &mut BTreeMap<StandardDeclarationKey, usize>,
) {
    for decl in decls
        .iter()
        .filter(|decl| is_embedded_standard_declaration(*decl))
    {
        *counts.entry(standard_declaration_key(decl)).or_insert(0) += 1;
    }
}

trait StandardDeclaration {
    fn module_name(&self) -> Option<&str>;
    fn span(&self) -> &SourceSpan;
    fn declaration_kind(&self) -> StandardDeclarationKind;
    fn declaration_name(&self) -> String;
}

macro_rules! impl_named_standard_declaration {
    ($ty:ty, $kind:expr) => {
        impl StandardDeclaration for $ty {
            fn module_name(&self) -> Option<&str> {
                self.module_name.as_deref()
            }

            fn span(&self) -> &SourceSpan {
                &self.span
            }

            fn declaration_kind(&self) -> StandardDeclarationKind {
                $kind
            }

            fn declaration_name(&self) -> String {
                self.name.clone().unwrap_or_default()
            }
        }
    };
}

impl StandardDeclaration for UseDecl {
    fn module_name(&self) -> Option<&str> {
        self.module_name.as_deref()
    }

    fn span(&self) -> &SourceSpan {
        &self.span
    }

    fn declaration_kind(&self) -> StandardDeclarationKind {
        StandardDeclarationKind::Use
    }

    fn declaration_name(&self) -> String {
        format!(
            "{}:{}:{}",
            self.package.as_deref().unwrap_or_default(),
            self.name,
            self.alias
        )
    }
}

impl StandardDeclaration for PublicAlias {
    fn module_name(&self) -> Option<&str> {
        self.module_name.as_deref()
    }

    fn span(&self) -> &SourceSpan {
        &self.span
    }

    fn declaration_kind(&self) -> StandardDeclarationKind {
        match self.kind {
            PublicAliasKind::Function => StandardDeclarationKind::FunctionAlias,
            PublicAliasKind::Type => StandardDeclarationKind::TypeAlias,
            PublicAliasKind::Schema => StandardDeclarationKind::SchemaAlias,
        }
    }

    fn declaration_name(&self) -> String {
        self.name.clone().unwrap_or_default()
    }
}

impl_named_standard_declaration!(EffectDecl, StandardDeclarationKind::Effect);
impl_named_standard_declaration!(HandlerDecl, StandardDeclarationKind::Handler);
impl_named_standard_declaration!(TypeDecl, StandardDeclarationKind::Type);
impl_named_standard_declaration!(SchemaDecl, StandardDeclarationKind::Schema);
impl_named_standard_declaration!(CodecDecl, StandardDeclarationKind::Codec);

impl StandardDeclaration for Function {
    fn module_name(&self) -> Option<&str> {
        self.module_name.as_deref()
    }

    fn span(&self) -> &SourceSpan {
        &self.span
    }

    fn declaration_kind(&self) -> StandardDeclarationKind {
        match self.kind {
            FunctionKind::Function => StandardDeclarationKind::Function,
            FunctionKind::Test => StandardDeclarationKind::Test,
        }
    }

    fn declaration_name(&self) -> String {
        self.name.clone().unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum StandardDeclarationKind {
    Use,
    FunctionAlias,
    TypeAlias,
    SchemaAlias,
    Effect,
    Handler,
    Type,
    Schema,
    Codec,
    Function,
    Test,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct StandardDeclarationKey {
    kind: StandardDeclarationKind,
    module_name: Option<String>,
    name: String,
    file: String,
    start: usize,
    end: usize,
}

fn standard_declaration_key(decl: &dyn StandardDeclaration) -> StandardDeclarationKey {
    let span = decl.span();
    StandardDeclarationKey {
        kind: decl.declaration_kind(),
        module_name: decl.module_name().map(str::to_string),
        name: decl.declaration_name(),
        file: span.file.as_str().to_string(),
        start: span.start.offset,
        end: span.end.offset,
    }
}

fn filter_module_declarations(
    module: &SurfaceModule,
    mut keep: impl FnMut(&dyn StandardDeclaration) -> bool,
) -> SurfaceModule {
    SurfaceModule {
        module: module.module.clone(),
        uses: module
            .uses
            .iter()
            .filter(|decl| keep(*decl))
            .cloned()
            .collect(),
        aliases: module
            .aliases
            .iter()
            .filter(|decl| keep(*decl))
            .cloned()
            .collect(),
        effects: module
            .effects
            .iter()
            .filter(|decl| keep(*decl))
            .cloned()
            .collect(),
        handlers: module
            .handlers
            .iter()
            .filter(|decl| keep(*decl))
            .cloned()
            .collect(),
        types: module
            .types
            .iter()
            .filter(|decl| keep(*decl))
            .cloned()
            .collect(),
        schemas: module
            .schemas
            .iter()
            .filter(|decl| keep(*decl))
            .cloned()
            .collect(),
        codecs: module
            .codecs
            .iter()
            .filter(|decl| keep(*decl))
            .cloned()
            .collect(),
        functions: module
            .functions
            .iter()
            .filter(|decl| keep(*decl))
            .cloned()
            .collect(),
    }
}

fn is_embedded_standard_declaration(decl: &dyn StandardDeclaration) -> bool {
    let Some(module_name) = decl.module_name() else {
        return false;
    };
    standard_module_name_from_bundle_path(decl.span().file.as_str()).as_deref() == Some(module_name)
}

fn is_standard_module_name(module_name: Option<&str>) -> bool {
    module_name.is_some_and(|module_name| module_name.starts_with("std::"))
}

pub(crate) fn ordinary_function_signatures(
    module: &SurfaceModule,
    effects: &[EffectSignature],
    adts: &AdtRegistry,
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Vec<FunctionSignature> {
    module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(|function| {
            let name = function.name.clone()?;
            let (params, variadic) = function_signature_params(function);
            let params = params
                .into_iter()
                .map(|ty| {
                    canonicalize_type_effects(
                        ty,
                        &module.uses,
                        function.module_name.as_deref(),
                        effects,
                        adts,
                        companion_effect_access_targets,
                    )
                })
                .collect();
            let variadic = variadic.map(|ty| {
                canonicalize_type_effects(
                    ty,
                    &module.uses,
                    function.module_name.as_deref(),
                    effects,
                    adts,
                    companion_effect_access_targets,
                )
            });
            let return_type = canonicalize_type_effects(
                parse_type_or_unknown(function.return_type.as_deref()),
                &module.uses,
                function.module_name.as_deref(),
                effects,
                adts,
                companion_effect_access_targets,
            );
            Some(FunctionSignature {
                target_name: crate::standard_symbols::standard_function_link_name(
                    function.module_name.as_deref(),
                    &name,
                ),
                name,
                module_name: function.module_name.clone(),
                visibility: function.visibility,
                params,
                variadic,
                return_type,
                effects: canonical_declared_effects(
                    function.effects.clone().unwrap_or_default(),
                    &module.uses,
                    function.module_name.as_deref(),
                    effects,
                    companion_effect_access_targets,
                ),
                node_id: function.node_id,
                span: function.span.clone(),
            })
        })
        .collect()
}

fn canonicalize_type_effects(
    ty: Type,
    uses: &[UseDecl],
    current_module: Option<&str>,
    effects: &[EffectSignature],
    adts: &AdtRegistry,
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Type {
    match ty {
        Type::Named { name, args } => Type::Named {
            name: adts
                .descriptor_for_type_path(&name, args.len(), current_module, uses)
                .map(|descriptor| descriptor.type_name.clone())
                .unwrap_or(name),
            args: args
                .into_iter()
                .map(|arg| {
                    canonicalize_type_effects(
                        arg,
                        uses,
                        current_module,
                        effects,
                        adts,
                        companion_effect_access_targets,
                    )
                })
                .collect(),
        },
        Type::Record(fields) => Type::Record(
            fields
                .into_iter()
                .map(|(name, ty)| {
                    (
                        name,
                        canonicalize_type_effects(
                            ty,
                            uses,
                            current_module,
                            effects,
                            adts,
                            companion_effect_access_targets,
                        ),
                    )
                })
                .collect(),
        ),
        Type::Function {
            params,
            variadic,
            return_type,
            effects: declared,
        } => Type::Function {
            params: params
                .into_iter()
                .map(|param| {
                    canonicalize_type_effects(
                        param,
                        uses,
                        current_module,
                        effects,
                        adts,
                        companion_effect_access_targets,
                    )
                })
                .collect(),
            variadic: variadic
                .map(|ty| {
                    canonicalize_type_effects(
                        *ty,
                        uses,
                        current_module,
                        effects,
                        adts,
                        companion_effect_access_targets,
                    )
                })
                .map(Box::new),
            return_type: Box::new(canonicalize_type_effects(
                *return_type,
                uses,
                current_module,
                effects,
                adts,
                companion_effect_access_targets,
            )),
            effects: canonical_declared_effects(
                declared,
                uses,
                current_module,
                effects,
                companion_effect_access_targets,
            ),
        },
        Type::Unknown => Type::Unknown,
    }
}

fn canonical_declared_effects(
    declared: Vec<String>,
    uses: &[UseDecl],
    current_module: Option<&str>,
    effects: &[EffectSignature],
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Vec<String> {
    let mut canonical = Vec::new();
    for effect in declared {
        if effect.starts_with("...") {
            push_unique_effect(&mut canonical, &effect);
            continue;
        }
        let segments = effect.split("::").map(str::to_string).collect::<Vec<_>>();
        let label = canonical_user_effect_label(
            &segments,
            uses,
            current_module,
            effects,
            companion_effect_access_targets,
        )
        .unwrap_or(effect);
        push_unique_effect(&mut canonical, &label);
    }
    canonical
}

fn effect_signatures(module: &SurfaceModule) -> Vec<EffectSignature> {
    module
        .effects
        .iter()
        .filter_map(|effect| {
            let name = effect.name.clone()?;
            let qualified_name = if let Some(module_name) = &effect.module_name {
                format!("{module_name}::{name}")
            } else {
                name.clone()
            };
            Some(EffectSignature {
                name,
                qualified_name,
                module_name: effect.module_name.clone(),
                visibility: effect.visibility,
                span: effect.span.clone(),
                operations: effect
                    .operations
                    .iter()
                    .filter_map(|operation| {
                        Some(EffectOperationSignature {
                            name: operation.name.clone()?,
                            params: operation
                                .params
                                .iter()
                                .map(|param| parse_type_or_unknown(param.ty.as_deref()))
                                .collect(),
                            return_type: parse_type_or_unknown(operation.return_type.as_deref()),
                            node_id: operation.node_id,
                            name_span: operation.name_span.clone(),
                        })
                    })
                    .collect(),
            })
        })
        .collect()
}

fn handler_signatures(
    module: &SurfaceModule,
    effects: &[EffectSignature],
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Vec<HandlerSignature> {
    module
        .handlers
        .iter()
        .filter_map(|handler| {
            let name = handler.name.clone()?;
            let qualified_name = if let Some(module_name) = &handler.module_name {
                format!("{module_name}::{name}")
            } else {
                name.clone()
            };
            let effect = canonical_user_effect_label(
                &handler.effect,
                &module.uses,
                handler.module_name.as_deref(),
                effects,
                companion_effect_access_targets,
            )
            .unwrap_or_else(|| handler.effect.join("::"));
            Some(HandlerSignature {
                name,
                qualified_name,
                module_name: handler.module_name.clone(),
                visibility: handler.visibility,
                params: handler
                    .params
                    .iter()
                    .map(|param| parse_type_or_unknown(param.ty.as_deref()))
                    .collect(),
                effect,
                effects: canonical_declared_effects(
                    handler.effects.clone().unwrap_or_default(),
                    &module.uses,
                    handler.module_name.as_deref(),
                    effects,
                    companion_effect_access_targets,
                ),
                operation_clauses: handler
                    .operation_clauses
                    .iter()
                    .filter_map(|clause| {
                        Some(HandlerOperationClauseSignature {
                            operation: clause.operation.clone()?,
                            function: synthetic_handler_clause_function_name(
                                handler.name.as_deref().unwrap_or("missing"),
                                clause.operation.as_deref().unwrap_or("missing"),
                            ),
                            module_name: handler.module_name.clone(),
                        })
                    })
                    .collect(),
            })
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum EffectDependencyNode {
    Function(FunctionKey),
    PrivateHandler(String),
}

struct FunctionEffectContext<'a> {
    module: &'a SurfaceModule,
    functions: &'a [FunctionSignature],
    user_effects: &'a [EffectSignature],
    handlers: &'a [HandlerSignature],
    effects_by_function: &'a BTreeMap<(Option<String>, String), Vec<String>>,
    effects_by_module_path: &'a BTreeMap<(String, String), (Vec<String>, Visibility)>,
    companion_access_targets: &'a BTreeMap<String, String>,
    companion_effect_access_targets: &'a BTreeMap<String, CompanionAccessTarget>,
}

struct HandlerEffectContext<'a> {
    module: &'a SurfaceModule,
    user_effects: &'a [EffectSignature],
    functions: &'a [FunctionSignature],
    effects_by_function: &'a EffectsByFunction,
    effects_by_module_path: &'a EffectsByModulePath,
    handlers: &'a [HandlerSignature],
    companion_access_targets: &'a BTreeMap<String, String>,
    companion_effect_access_targets: &'a BTreeMap<String, CompanionAccessTarget>,
}

fn infer_function_and_private_handler_effects(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    user_effects: &[EffectSignature],
    handlers: &mut [HandlerSignature],
) {
    EffectInference::new(module, functions, user_effects, handlers).run();
}

type EffectsByFunction = BTreeMap<(Option<String>, String), Vec<String>>;
type EffectsByModulePath = BTreeMap<(String, String), (Vec<String>, Visibility)>;

struct EffectInference<'a> {
    module: &'a SurfaceModule,
    functions: &'a mut [FunctionSignature],
    user_effects: &'a [EffectSignature],
    handlers: &'a mut [HandlerSignature],
    graph: EffectDependencyGraph,
    companion_access_targets: BTreeMap<String, String>,
    companion_effect_access_targets: BTreeMap<String, CompanionAccessTarget>,
    clause_companion_access_targets: BTreeMap<String, String>,
    effects_by_function: EffectsByFunction,
    effects_by_module_path: EffectsByModulePath,
    handler_index: BTreeMap<String, usize>,
    function_index: BTreeMap<FunctionKey, usize>,
    function_ast_by_key: BTreeMap<FunctionKey, &'a Function>,
    queue: VecDeque<EffectDependencyNode>,
    queued: BTreeSet<EffectDependencyNode>,
    evaluated: BTreeSet<EffectDependencyNode>,
}

impl<'a> EffectInference<'a> {
    fn new(
        module: &'a SurfaceModule,
        functions: &'a mut [FunctionSignature],
        user_effects: &'a [EffectSignature],
        handlers: &'a mut [HandlerSignature],
    ) -> Self {
        let graph = effect_dependency_graph(module, functions, user_effects, handlers);
        let (effects_by_function, effects_by_module_path) = effect_lookup_maps(functions);
        let queue = graph.ordered_nodes.iter().cloned().collect();
        let queued = graph.nodes.clone();
        Self {
            module,
            companion_access_targets: companion_function_access_targets(module),
            companion_effect_access_targets: companion_access_target_infos(module),
            clause_companion_access_targets: companion_access_targets_for_signatures(functions),
            effects_by_function,
            effects_by_module_path,
            handler_index: handler_signature_index(handlers),
            function_index: function_signature_index(functions),
            function_ast_by_key: function_ast_index(module),
            queue,
            queued,
            evaluated: BTreeSet::new(),
            graph,
            functions,
            user_effects,
            handlers,
        }
    }

    fn run(mut self) {
        while let Some(node) = self.queue.pop_front() {
            self.queued.remove(&node);
            let is_reevaluation = self.evaluated.contains(&node);
            let Some(changed) = self.evaluate_node(&node, is_reevaluation) else {
                continue;
            };
            self.evaluated.insert(node.clone());
            if changed {
                self.enqueue_dependents(&node);
            }
        }
    }

    fn evaluate_node(
        &mut self,
        node: &EffectDependencyNode,
        is_reevaluation: bool,
    ) -> Option<bool> {
        if is_reevaluation {
            #[cfg(test)]
            effect_inference_counters::record_changed_reevaluation();
        }
        match node {
            EffectDependencyNode::Function(function_key) => self.evaluate_function(function_key),
            EffectDependencyNode::PrivateHandler(qualified_name) => {
                self.evaluate_handler(qualified_name)
            }
        }
    }

    fn evaluate_function(&mut self, function_key: &FunctionKey) -> Option<bool> {
        let function = self.function_ast_by_key.get(function_key).copied()?;
        let inferred = collect_function_body_effects(function, &self.function_context());
        let changed = self.effects_by_function.get(function_key) != Some(&inferred);
        if changed {
            self.update_function_effects(function_key, inferred);
        }
        Some(changed)
    }

    fn evaluate_handler(&mut self, qualified_name: &str) -> Option<bool> {
        let index = self.handler_index.get(qualified_name).copied()?;
        let inferred = collect_private_handler_effects(
            &self.handlers[index],
            &HandlerEffectContext {
                module: self.module,
                user_effects: self.user_effects,
                functions: self.functions,
                effects_by_function: &self.effects_by_function,
                effects_by_module_path: &self.effects_by_module_path,
                handlers: self.handlers,
                companion_access_targets: &self.clause_companion_access_targets,
                companion_effect_access_targets: &self.companion_effect_access_targets,
            },
        );
        let changed = self.handlers[index].effects != inferred;
        if changed {
            self.handlers[index].effects = inferred;
        }
        Some(changed)
    }

    fn function_context(&self) -> FunctionEffectContext<'_> {
        FunctionEffectContext {
            module: self.module,
            functions: self.functions,
            user_effects: self.user_effects,
            handlers: self.handlers,
            effects_by_function: &self.effects_by_function,
            effects_by_module_path: &self.effects_by_module_path,
            companion_access_targets: &self.companion_access_targets,
            companion_effect_access_targets: &self.companion_effect_access_targets,
        }
    }

    fn update_function_effects(&mut self, function_key: &FunctionKey, inferred: Vec<String>) {
        self.effects_by_function
            .insert(function_key.clone(), inferred.clone());
        if let Some(module_name) = &function_key.0 {
            let visibility = self
                .function_index
                .get(function_key)
                .map(|index| self.functions[*index].visibility)
                .unwrap_or(Visibility::Private);
            self.effects_by_module_path.insert(
                (module_name.clone(), function_key.1.clone()),
                (inferred.clone(), visibility),
            );
        }
        if let Some(index) = self.function_index.get(function_key).copied() {
            self.functions[index].effects = inferred;
        }
    }

    fn enqueue_dependents(&mut self, node: &EffectDependencyNode) {
        let Some(dependents) = self.graph.dependents.get(node) else {
            return;
        };
        for dependent in dependents {
            if self.queued.insert(dependent.clone()) {
                self.queue.push_back(dependent.clone());
            }
        }
    }
}

fn effect_lookup_maps(functions: &[FunctionSignature]) -> (EffectsByFunction, EffectsByModulePath) {
    let effects_by_function = functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.effects.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let effects_by_module_path = functions
        .iter()
        .filter_map(|function| {
            Some((
                (function.module_name.clone()?, function.name.clone()),
                (function.effects.clone(), function.visibility),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    (effects_by_function, effects_by_module_path)
}

fn handler_signature_index(handlers: &[HandlerSignature]) -> BTreeMap<String, usize> {
    handlers
        .iter()
        .enumerate()
        .map(|(index, handler)| (handler.qualified_name.clone(), index))
        .collect()
}

fn function_signature_index(functions: &[FunctionSignature]) -> BTreeMap<FunctionKey, usize> {
    functions
        .iter()
        .enumerate()
        .map(|(index, function)| ((function.module_name.clone(), function.name.clone()), index))
        .collect()
}

fn function_ast_index(module: &SurfaceModule) -> BTreeMap<FunctionKey, &Function> {
    module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(|function| {
            Some((
                (function.module_name.clone(), function.name.clone()?),
                function,
            ))
        })
        .collect()
}

struct EffectDependencyGraph {
    nodes: BTreeSet<EffectDependencyNode>,
    ordered_nodes: Vec<EffectDependencyNode>,
    dependents: BTreeMap<EffectDependencyNode, BTreeSet<EffectDependencyNode>>,
}

impl EffectDependencyGraph {
    fn new() -> Self {
        Self {
            nodes: BTreeSet::new(),
            ordered_nodes: Vec::new(),
            dependents: BTreeMap::new(),
        }
    }

    fn insert_node(&mut self, node: EffectDependencyNode) {
        if self.nodes.insert(node.clone()) {
            self.ordered_nodes.push(node);
        }
    }

    fn insert_dependency(
        &mut self,
        dependency: EffectDependencyNode,
        dependent: EffectDependencyNode,
    ) {
        self.dependents
            .entry(dependency)
            .or_default()
            .insert(dependent);
    }
}

fn effect_dependency_graph(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
    user_effects: &[EffectSignature],
    handlers: &[HandlerSignature],
) -> EffectDependencyGraph {
    let companion_access_targets = companion_function_access_targets(module);
    let companion_effect_access_targets = companion_access_target_infos(module);
    let (effects_by_function, effects_by_module_path) = effect_lookup_maps(functions);
    let module_private_handlers = module
        .handlers
        .iter()
        .filter(|handler| handler.visibility != Visibility::Public)
        .filter_map(|handler| {
            let name = handler.name.as_deref()?;
            Some(qualified_name(handler.module_name.as_deref(), name))
        })
        .collect::<BTreeSet<_>>();
    let context = FunctionEffectContext {
        module,
        functions,
        user_effects,
        handlers,
        effects_by_function: &effects_by_function,
        effects_by_module_path: &effects_by_module_path,
        companion_access_targets: &companion_access_targets,
        companion_effect_access_targets: &companion_effect_access_targets,
    };
    let mut graph = EffectDependencyGraph::new();
    for function in module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
    {
        insert_function_effect_dependencies(&mut graph, function, &context);
    }
    for handler in handlers
        .iter()
        .filter(|handler| handler.visibility != Visibility::Public)
        .filter(|handler| module_private_handlers.contains(&handler.qualified_name))
    {
        insert_handler_effect_dependencies(&mut graph, handler, module, &context);
    }
    graph
}

fn qualified_name(module_name: Option<&str>, name: &str) -> String {
    module_name.map_or_else(
        || name.to_string(),
        |module_name| format!("{module_name}::{name}"),
    )
}

fn insert_function_effect_dependencies(
    graph: &mut EffectDependencyGraph,
    function: &Function,
    context: &FunctionEffectContext<'_>,
) {
    let Some(name) = &function.name else {
        return;
    };
    #[cfg(test)]
    effect_inference_counters::record_dependency_discovery_scan();
    let node = EffectDependencyNode::Function((function.module_name.clone(), name.clone()));
    graph.insert_node(node.clone());
    for dependency in function_effect_dependencies(function, context) {
        graph.insert_dependency(dependency, node.clone());
    }
}

fn insert_handler_effect_dependencies(
    graph: &mut EffectDependencyGraph,
    handler: &HandlerSignature,
    module: &SurfaceModule,
    context: &FunctionEffectContext<'_>,
) {
    #[cfg(test)]
    effect_inference_counters::record_dependency_discovery_scan();
    let node = EffectDependencyNode::PrivateHandler(handler.qualified_name.clone());
    graph.insert_node(node.clone());
    let Some(decl) = module.handlers.iter().find(|decl| {
        decl.name.as_deref() == Some(handler.name.as_str())
            && decl.module_name == handler.module_name
    }) else {
        return;
    };
    let mut bindings = decl
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| {
            Binding::new(
                param.name.clone(),
                handler.params.get(index).cloned().unwrap_or(Type::Unknown),
            )
        })
        .collect::<Vec<_>>();
    for clause in &decl.operation_clauses {
        let binding_count = bindings.len();
        bindings.extend(
            clause
                .params
                .iter()
                .map(|param| Binding::new(param.name.clone(), Type::Unknown)),
        );
        let expr_context = ExprEffectContext {
            uses: &module.uses,
            current_module: handler.module_name.as_deref(),
            bindings: &bindings,
            functions: context.functions,
            effects_by_function: context.effects_by_function,
            effects_by_module_path: context.effects_by_module_path,
            companion_access_targets: context.companion_access_targets,
            companion_effect_access_targets: context.companion_effect_access_targets,
            user_effects: context.user_effects,
            handlers: context.handlers,
        };
        let mut dependencies = BTreeSet::new();
        collect_expr_effect_dependencies(&clause.body, &expr_context, &mut dependencies);
        for dependency in dependencies {
            graph.insert_dependency(dependency, node.clone());
        }
        bindings.truncate(binding_count);
    }
}

fn collect_private_handler_effects(
    handler: &HandlerSignature,
    context: &HandlerEffectContext<'_>,
) -> Vec<String> {
    #[cfg(test)]
    effect_inference_counters::record_handler_operation_clause_evaluation();
    let Some(decl) = context.module.handlers.iter().find(|decl| {
        decl.name.as_deref() == Some(handler.name.as_str())
            && decl.module_name == handler.module_name
    }) else {
        return Vec::new();
    };
    let Some(effect) = context
        .user_effects
        .iter()
        .find(|effect| effect.qualified_name == handler.effect)
    else {
        return Vec::new();
    };
    let mut inferred = Vec::new();
    for clause in &decl.operation_clauses {
        let Some(operation_name) = clause.operation.as_deref() else {
            continue;
        };
        let Some(operation) = effect
            .operations
            .iter()
            .find(|operation| operation.name == operation_name)
        else {
            continue;
        };
        let mut bindings = decl
            .params
            .iter()
            .enumerate()
            .map(|(index, param)| {
                Binding::new(
                    param.name.clone(),
                    handler.params.get(index).cloned().unwrap_or(Type::Unknown),
                )
            })
            .collect::<Vec<_>>();
        bindings.extend(clause.params.iter().enumerate().map(|(index, param)| {
            Binding::new(
                param.name.clone(),
                operation
                    .params
                    .get(index)
                    .cloned()
                    .unwrap_or(Type::Unknown),
            )
        }));
        let expr_context = ExprEffectContext {
            uses: &context.module.uses,
            current_module: handler.module_name.as_deref(),
            bindings: &bindings,
            functions: context.functions,
            effects_by_function: context.effects_by_function,
            effects_by_module_path: context.effects_by_module_path,
            companion_access_targets: context.companion_access_targets,
            companion_effect_access_targets: context.companion_effect_access_targets,
            user_effects: context.user_effects,
            handlers: context.handlers,
        };
        collect_expr_effects(&clause.body, &expr_context, &mut inferred);
    }
    inferred
}

fn collect_function_body_effects(
    function: &Function,
    context: &FunctionEffectContext<'_>,
) -> Vec<String> {
    #[cfg(test)]
    effect_inference_counters::record_function_body_collection();
    let mut bindings = function
        .params
        .iter()
        .map(|param| Binding::new(param.name.clone(), function_body_param_type(param)))
        .collect::<Vec<_>>();
    let function_key = (
        function.module_name.clone(),
        function.name.clone().unwrap_or_default(),
    );
    let mut inferred = context
        .effects_by_function
        .get(&function_key)
        .cloned()
        .unwrap_or_default();
    for line in &function.body {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let expr_context = ExprEffectContext {
                    uses: &context.module.uses,
                    current_module: function.module_name.as_deref(),
                    bindings: &bindings,
                    functions: context.functions,
                    effects_by_function: context.effects_by_function,
                    effects_by_module_path: context.effects_by_module_path,
                    companion_access_targets: context.companion_access_targets,
                    companion_effect_access_targets: context.companion_effect_access_targets,
                    user_effects: context.user_effects,
                    handlers: context.handlers,
                };
                collect_expr_effects(expr, &expr_context, &mut inferred);
                let ty = parse_type_or_unknown(annotation.as_deref());
                collect_pattern_bindings(pattern, &ty, &mut bindings);
            }
            BodyLineKind::Expr { expr } => {
                let expr_context = ExprEffectContext {
                    uses: &context.module.uses,
                    current_module: function.module_name.as_deref(),
                    bindings: &bindings,
                    functions: context.functions,
                    effects_by_function: context.effects_by_function,
                    effects_by_module_path: context.effects_by_module_path,
                    companion_access_targets: context.companion_access_targets,
                    companion_effect_access_targets: context.companion_effect_access_targets,
                    user_effects: context.user_effects,
                    handlers: context.handlers,
                };
                collect_expr_effects(expr, &expr_context, &mut inferred);
            }
        }
    }
    inferred
}

fn function_effect_dependencies(
    function: &Function,
    context: &FunctionEffectContext<'_>,
) -> BTreeSet<EffectDependencyNode> {
    let mut dependencies = BTreeSet::new();
    let mut bindings = function
        .params
        .iter()
        .map(|param| Binding::new(param.name.clone(), function_body_param_type(param)))
        .collect::<Vec<_>>();
    for line in &function.body {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let expr_context = ExprEffectContext {
                    uses: &context.module.uses,
                    current_module: function.module_name.as_deref(),
                    bindings: &bindings,
                    functions: context.functions,
                    effects_by_function: context.effects_by_function,
                    effects_by_module_path: context.effects_by_module_path,
                    companion_access_targets: context.companion_access_targets,
                    companion_effect_access_targets: context.companion_effect_access_targets,
                    user_effects: context.user_effects,
                    handlers: context.handlers,
                };
                collect_expr_effect_dependencies(expr, &expr_context, &mut dependencies);
                let ty = parse_type_or_unknown(annotation.as_deref());
                collect_pattern_bindings(pattern, &ty, &mut bindings);
            }
            BodyLineKind::Expr { expr } => {
                let expr_context = ExprEffectContext {
                    uses: &context.module.uses,
                    current_module: function.module_name.as_deref(),
                    bindings: &bindings,
                    functions: context.functions,
                    effects_by_function: context.effects_by_function,
                    effects_by_module_path: context.effects_by_module_path,
                    companion_access_targets: context.companion_access_targets,
                    companion_effect_access_targets: context.companion_effect_access_targets,
                    user_effects: context.user_effects,
                    handlers: context.handlers,
                };
                collect_expr_effect_dependencies(expr, &expr_context, &mut dependencies);
            }
        }
    }
    dependencies
}

pub(crate) fn canonical_user_effect_label(
    segments: &[String],
    uses: &[UseDecl],
    current_module: Option<&str>,
    effects: &[EffectSignature],
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Option<String> {
    match segments {
        [name] => effects
            .iter()
            .find(|effect| effect.name == *name && effect.module_name.as_deref() == current_module)
            .map(|effect| effect.qualified_name.clone()),
        [_, .., name] => {
            let use_decl =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
            effects
                .iter()
                .find(|effect| {
                    effect.name == *name
                        && effect.module_name.as_deref() == Some(use_decl.name.as_str())
                        && imported_effect_is_visible(
                            use_decl,
                            current_module,
                            use_decl.name.as_str(),
                            effect.visibility,
                            companion_effect_access_targets,
                        )
                })
                .map(|effect| effect.qualified_name.clone())
        }
        _ => None,
    }
}

pub(crate) fn imported_effect_is_visible(
    use_decl: &UseDecl,
    current_module: Option<&str>,
    target_module: &str,
    visibility: Visibility,
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> bool {
    visibility == Visibility::Public
        || (use_decl.package.is_none()
            && current_module.is_some_and(|current_module| {
                (current_module.starts_with("std::") && target_module.starts_with("std::"))
                    || companion_effect_access_targets
                        .get(current_module)
                        .is_some_and(|access| access.target_module == target_module)
            }))
}

fn function_signature_params(function: &veln_ast::Function) -> (Vec<Type>, Option<Type>) {
    let mut params = Vec::new();
    let mut variadic = None;
    for param in &function.params {
        let ty = parse_type_or_unknown(param.ty.as_deref());
        if param.is_variadic {
            variadic = Some(ty);
        } else {
            params.push(ty);
        }
    }
    (params, variadic)
}

fn infer_private_function_body_return_types(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    adts: &AdtRegistry,
) {
    let mut changed = true;
    while changed {
        changed = false;
        let signatures_by_path = signatures_by_path(functions);
        let omitted_private_returns = omitted_private_returns_that_can_change(module, functions);
        if omitted_private_returns.is_empty() {
            return;
        }
        let returns_by_path = returns_by_path(functions);
        for function in module.functions.iter().filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && private_function_key(function)
                    .is_some_and(|key| omitted_private_returns.contains(&key))
        }) {
            let Some(name) = &function.name else {
                continue;
            };
            let key = (function.module_name.clone(), name.clone());
            let inferred = infer_private_function_tail_type(
                function,
                &module.uses,
                &signatures_by_path,
                &returns_by_path,
                adts,
            );
            if inferred == Type::Unknown {
                continue;
            }
            let Some(signature) = functions
                .iter_mut()
                .find(|signature| signature.module_name == key.0 && signature.name == key.1)
            else {
                continue;
            };
            if signature.return_type == inferred {
                continue;
            }
            if !type_has_unknown(&signature.return_type) {
                continue;
            }
            signature.return_type = inferred;
            changed = true;
        }
    }
}

fn infer_private_function_call_site_signature_types(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    adts: &AdtRegistry,
) {
    let function_by_path = module
        .functions
        .iter()
        .filter_map(|function| {
            Some((
                (function.module_name.clone(), function.name.clone()?),
                function,
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let initial_omitted_private_slots = omitted_private_slots_that_can_change(module, functions);
    if initial_omitted_private_slots.is_empty() {
        return;
    }
    let private_references = private_reference_map(
        module,
        &function_by_path,
        &modules_with_private_slot_omissions(&initial_omitted_private_slots),
        &initial_omitted_private_slots
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
    );
    let contributors = private_call_site_constraint_contributors(
        module,
        &initial_omitted_private_slots,
        &private_references,
    );
    let mut changed = true;
    while changed {
        changed = false;
        let omitted_private_slots = omitted_private_slots_that_can_change(module, functions);
        if omitted_private_slots.is_empty() {
            return;
        }
        let signatures_by_path = signatures_by_path_with_aliases(module, functions);
        let returns_by_path = returns_by_path(functions);
        for function in module.functions.iter().filter(|function| {
            function_key(function).is_some_and(|key| contributors.contains(&key))
        }) {
            collect_private_call_site_constraints(
                function,
                &mut PrivateCallSiteConstraintContext {
                    uses: &module.uses,
                    function_by_path: &function_by_path,
                    omitted_private_slots: &omitted_private_slots,
                    signatures_by_path: &signatures_by_path,
                    returns_by_path: &returns_by_path,
                    functions,
                    adts,
                    changed: &mut changed,
                },
            );
        }
    }
}

fn function_key(function: &Function) -> Option<FunctionKey> {
    Some((function.module_name.clone(), function.name.clone()?))
}

fn private_function_key(function: &Function) -> Option<FunctionKey> {
    Some((function.module_name.clone(), function.name.clone()?))
}

fn signature_for_key<'a>(
    functions: &'a [FunctionSignature],
    key: &FunctionKey,
) -> Option<&'a FunctionSignature> {
    functions
        .iter()
        .find(|signature| signature.module_name == key.0 && signature.name == key.1)
}

fn omitted_private_returns_that_can_change(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> BTreeSet<FunctionKey> {
    module
        .functions
        .iter()
        .filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && function.return_type.is_none()
        })
        .filter_map(|function| {
            let key = private_function_key(function)?;
            let can_change = signature_for_key(functions, &key)
                .is_some_and(|signature| type_has_unknown(&signature.return_type));
            can_change.then_some(key)
        })
        .collect()
}

fn omitted_private_slots_that_can_change(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> PrivateSlotMap {
    module
        .functions
        .iter()
        .filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && function.name.is_some()
        })
        .filter_map(|function| {
            let key = private_function_key(function)?;
            let signature = signature_for_key(functions, &key)?;
            let omitted_params = function
                .params
                .iter()
                .enumerate()
                .map(|(index, param)| {
                    if !parameter_annotation_is_omitted(param) {
                        return false;
                    }
                    if param.is_variadic {
                        signature.variadic.as_ref().is_some_and(type_has_unknown)
                    } else {
                        signature.params.get(index).is_some_and(type_has_unknown)
                    }
                })
                .collect::<Vec<_>>();
            let omitted_return =
                function.return_type.is_none() && type_has_unknown(&signature.return_type);
            (omitted_params.iter().any(|omitted| *omitted) || omitted_return)
                .then_some((key, (omitted_params, omitted_return)))
        })
        .collect()
}

fn modules_with_private_slot_omissions(
    omitted_private_slots: &PrivateSlotMap,
) -> BTreeSet<Option<String>> {
    omitted_private_slots
        .keys()
        .map(|key| key.0.clone())
        .collect()
}

fn modules_with_private_return_omissions(
    omitted_private_returns: &BTreeSet<FunctionKey>,
) -> BTreeSet<Option<String>> {
    omitted_private_returns
        .iter()
        .map(|key| key.0.clone())
        .collect()
}

fn omitted_private_returns_requiring_prelude_pass(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> BTreeSet<FunctionKey> {
    module
        .functions
        .iter()
        .filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && function.return_type.is_none()
        })
        .filter_map(|function| {
            let key = private_function_key(function)?;
            let signature = signature_for_key(functions, &key)?;
            (type_has_unknown(&signature.return_type)
                || private_tail_can_use_expected(function, &signature.return_type, uses, adts))
            .then_some(key)
        })
        .collect()
}

fn private_reference_map(
    module: &SurfaceModule,
    function_by_path: &FunctionAstMap<'_>,
    modules_with_omitted_slots: &BTreeSet<Option<String>>,
    omitted_private_keys: &BTreeSet<FunctionKey>,
) -> PrivateReferenceMap {
    let candidates_by_module = private_reference_candidates_by_module(omitted_private_keys);
    module
        .functions
        .iter()
        .filter(|function| modules_with_omitted_slots.contains(&function.module_name))
        .filter(|function| {
            private_function_needs_reference_index(
                function,
                function_by_path,
                &candidates_by_module,
                omitted_private_keys,
            )
        })
        .filter_map(|function| {
            let key = function_key(function)?;
            let mut references = BTreeSet::new();
            #[cfg(test)]
            private_inference_counters::record_private_reference_index_scan();
            collect_private_function_references(function, function_by_path, &mut references);
            Some((key, references))
        })
        .collect()
}

fn private_reference_candidates_by_module(
    omitted_private_keys: &BTreeSet<FunctionKey>,
) -> BTreeMap<Option<String>, BTreeSet<String>> {
    let mut candidates: BTreeMap<Option<String>, BTreeSet<String>> = BTreeMap::new();
    for (module_name, name) in omitted_private_keys {
        candidates
            .entry(module_name.clone())
            .or_default()
            .insert(name.clone());
    }
    candidates
}

fn private_function_needs_reference_index(
    function: &Function,
    function_by_path: &FunctionAstMap<'_>,
    candidates_by_module: &BTreeMap<Option<String>, BTreeSet<String>>,
    omitted_private_keys: &BTreeSet<FunctionKey>,
) -> bool {
    let Some(key) = function_key(function) else {
        return false;
    };
    if omitted_private_keys.contains(&key) {
        return true;
    }
    let Some(candidates) = candidates_by_module.get(&function.module_name) else {
        return false;
    };
    #[cfg(test)]
    private_inference_counters::record_private_reference_candidate_scan();
    private_function_mentions_candidate(function, function_by_path, candidates)
}

fn private_function_mentions_candidate(
    function: &Function,
    function_by_path: &FunctionAstMap<'_>,
    candidates: &BTreeSet<String>,
) -> bool {
    let current_module = function.module_name.as_deref();
    let mut bindings = private_reference_initial_bindings(function);
    for line in &function.body {
        if private_line_mentions_candidate(
            line,
            current_module,
            function_by_path,
            candidates,
            &mut bindings,
        ) {
            return true;
        }
    }
    false
}

fn private_line_mentions_candidate(
    line: &BodyLine,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    candidates: &BTreeSet<String>,
    bindings: &mut Vec<Binding>,
) -> bool {
    match &line.kind {
        BodyLineKind::Let { pattern, expr, .. } => {
            let mentions = private_expr_mentions_candidate(
                expr,
                current_module,
                function_by_path,
                candidates,
                bindings,
            );
            let initializer_private_function =
                private_expr_reference_target(expr, current_module, function_by_path, bindings);
            collect_let_pattern_bindings(
                pattern,
                &Type::Unknown,
                initializer_private_function,
                bindings,
            );
            mentions
        }
        BodyLineKind::Expr { expr } => private_expr_mentions_candidate(
            expr,
            current_module,
            function_by_path,
            candidates,
            bindings,
        ),
    }
}

fn private_expr_mentions_candidate(
    expr: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    candidates: &BTreeSet<String>,
    bindings: &[Binding],
) -> bool {
    if private_expr_reference_target(expr, current_module, function_by_path, bindings)
        .is_some_and(|key| key.0.as_deref() == current_module && candidates.contains(&key.1))
    {
        return true;
    }
    match &expr.kind {
        ExprKind::List(items) => items.iter().any(|item| {
            private_expr_mentions_candidate(
                item,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }),
        ExprKind::Dict(entries) => entries.iter().any(|entry| {
            private_expr_mentions_candidate(
                &entry.key,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || private_expr_mentions_candidate(
                &entry.value,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }),
        ExprKind::Record(fields) => fields.iter().any(|field| {
            private_expr_mentions_candidate(
                &field.expr,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }),
        ExprKind::Call { callee, args } => {
            private_expr_mentions_candidate(
                callee,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || args.iter().any(|arg| {
                private_expr_mentions_candidate(
                    arg,
                    current_module,
                    function_by_path,
                    candidates,
                    bindings,
                )
            })
        }
        ExprKind::Perform { args, .. } => args.iter().any(|arg| {
            private_expr_mentions_candidate(
                arg,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }),
        ExprKind::Handle { body, args, .. } => {
            private_expr_mentions_candidate(
                body,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || args.iter().any(|arg| {
                private_expr_mentions_candidate(
                    arg,
                    current_module,
                    function_by_path,
                    candidates,
                    bindings,
                )
            })
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            private_expr_mentions_candidate(
                input,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || private_expr_mentions_candidate(
                base,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::FieldAccess { base: value, .. }
        | ExprKind::Try(value)
        | ExprKind::Prefix { expr: value, .. } => private_expr_mentions_candidate(
            value,
            current_module,
            function_by_path,
            candidates,
            bindings,
        ),
        ExprKind::Match { scrutinee, arms } => {
            private_expr_mentions_candidate(
                scrutinee,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || arms.iter().any(|arm| {
                let mut arm_bindings = bindings.to_vec();
                collect_private_reference_pattern_bindings(&arm.pattern, &mut arm_bindings);
                private_expr_mentions_candidate(
                    &arm.expr,
                    current_module,
                    function_by_path,
                    candidates,
                    &arm_bindings,
                )
            })
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            private_expr_mentions_candidate(
                condition,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || private_expr_mentions_candidate(
                then_branch,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || else_if_branches.iter().any(|branch| {
                private_expr_mentions_candidate(
                    &branch.condition,
                    current_module,
                    function_by_path,
                    candidates,
                    bindings,
                ) || private_expr_mentions_candidate(
                    &branch.expr,
                    current_module,
                    function_by_path,
                    candidates,
                    bindings,
                )
            }) || private_expr_mentions_candidate(
                else_branch,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }
        ExprKind::Binary { left, right, .. } => {
            private_expr_mentions_candidate(
                left,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || private_expr_mentions_candidate(
                right,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }
        ExprKind::NamePath(_)
        | ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => false,
    }
}

fn private_reference_initial_bindings(function: &Function) -> Vec<Binding> {
    function
        .params
        .iter()
        .map(|param| Binding::new(param.name.clone(), function_body_param_type(param)))
        .collect()
}

fn collect_private_reference_pattern_bindings(pattern: &Pattern, bindings: &mut Vec<Binding>) {
    match &pattern.kind {
        PatternKind::Binding(name) => bindings.push(Binding::new(name.clone(), Type::Unknown)),
        PatternKind::Record(fields) => {
            for field in fields {
                collect_private_reference_pattern_bindings(&field.pattern, bindings);
            }
        }
        PatternKind::Constructor { args, .. } => {
            for arg in args {
                collect_private_reference_pattern_bindings(arg, bindings);
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

fn collect_private_function_references(
    function: &Function,
    function_by_path: &FunctionAstMap<'_>,
    references: &mut BTreeSet<FunctionKey>,
) {
    let current_module = function.module_name.as_deref();
    let mut bindings = private_reference_initial_bindings(function);
    for line in &function.body {
        collect_private_line_references(
            line,
            current_module,
            function_by_path,
            references,
            &mut bindings,
        );
    }
}

fn collect_private_line_references(
    line: &BodyLine,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    references: &mut BTreeSet<FunctionKey>,
    bindings: &mut Vec<Binding>,
) {
    match &line.kind {
        BodyLineKind::Let { pattern, expr, .. } => {
            collect_private_expr_references(
                expr,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            let initializer_private_function =
                private_expr_reference_target(expr, current_module, function_by_path, bindings);
            collect_let_pattern_bindings(
                pattern,
                &Type::Unknown,
                initializer_private_function,
                bindings,
            );
        }
        BodyLineKind::Expr { expr } => collect_private_expr_references(
            expr,
            current_module,
            function_by_path,
            references,
            bindings,
        ),
    }
}

fn collect_private_expr_references(
    expr: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    references: &mut BTreeSet<FunctionKey>,
    bindings: &[Binding],
) {
    if let Some(key) =
        private_expr_reference_target(expr, current_module, function_by_path, bindings)
    {
        references.insert(key);
    }
    match &expr.kind {
        ExprKind::List(items) => {
            for item in items {
                collect_private_expr_references(
                    item,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_private_expr_references(
                    &entry.key,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
                collect_private_expr_references(
                    &entry.value,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Record(fields) => {
            for field in fields {
                collect_private_expr_references(
                    &field.expr,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Call { callee, args } => {
            collect_private_expr_references(
                callee,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            for arg in args {
                collect_private_expr_references(
                    arg,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_private_expr_references(
                    arg,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_private_expr_references(
                body,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            for arg in args {
                collect_private_expr_references(
                    arg,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_private_expr_references(
                input,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            collect_private_expr_references(
                base,
                current_module,
                function_by_path,
                references,
                bindings,
            );
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::FieldAccess { base: value, .. }
        | ExprKind::Try(value)
        | ExprKind::Prefix { expr: value, .. } => {
            collect_private_expr_references(
                value,
                current_module,
                function_by_path,
                references,
                bindings,
            );
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_private_expr_references(
                scrutinee,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            for arm in arms {
                let mut arm_bindings = bindings.to_vec();
                collect_private_reference_pattern_bindings(&arm.pattern, &mut arm_bindings);
                collect_private_expr_references(
                    &arm.expr,
                    current_module,
                    function_by_path,
                    references,
                    &arm_bindings,
                );
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_private_expr_references(
                condition,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            collect_private_expr_references(
                then_branch,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            for branch in else_if_branches {
                collect_private_expr_references(
                    &branch.condition,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
                collect_private_expr_references(
                    &branch.expr,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
            collect_private_expr_references(
                else_branch,
                current_module,
                function_by_path,
                references,
                bindings,
            );
        }
        ExprKind::Binary { left, right, .. } => {
            collect_private_expr_references(
                left,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            collect_private_expr_references(
                right,
                current_module,
                function_by_path,
                references,
                bindings,
            );
        }
        ExprKind::NamePath(_)
        | ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => {}
    }
}

fn private_expr_reference_target(
    expr: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    bindings: &[Binding],
) -> Option<FunctionKey> {
    let ExprKind::NamePath(segments) = &expr.kind else {
        return None;
    };
    private_reference_name_path_target(segments, current_module, function_by_path, bindings)
}

fn private_reference_name_path_target(
    segments: &[String],
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    bindings: &[Binding],
) -> Option<FunctionKey> {
    let [name] = segments else {
        return None;
    };
    if let Some(binding) = bindings.iter().rev().find(|binding| binding.name == *name) {
        return binding.private_function_value.clone();
    }
    private_name_path_target(segments, current_module, function_by_path)
}

fn signatures_by_path(functions: &[FunctionSignature]) -> FunctionSignatureMap {
    functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.clone(),
            )
        })
        .collect()
}

fn private_call_site_constraint_contributors(
    module: &SurfaceModule,
    omitted_private_slots: &PrivateSlotMap,
    private_references: &PrivateReferenceMap,
) -> BTreeSet<FunctionKey> {
    let modules_with_omitted_slots = omitted_private_slots
        .keys()
        .map(|key| key.0.clone())
        .collect::<BTreeSet<_>>();
    module
        .functions
        .iter()
        .filter(|function| modules_with_omitted_slots.contains(&function.module_name))
        .filter_map(|function| {
            let key = function_key(function)?;
            #[cfg(test)]
            private_inference_counters::record_call_site_discovery_scan();
            (omitted_private_slots.contains_key(&key)
                || private_references.get(&key).is_some_and(|references| {
                    references
                        .iter()
                        .any(|reference| omitted_private_slots.contains_key(reference))
                }))
            .then_some(key)
        })
        .collect()
}

fn signatures_by_path_with_aliases(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> FunctionSignatureMap {
    let mut signatures = signatures_by_path(functions);
    for alias in function_alias_signatures(module, functions) {
        let key = (alias.module_name.clone(), alias.name.clone());
        signatures.entry(key).or_insert(alias);
    }
    signatures
}

fn returns_by_path(functions: &[FunctionSignature]) -> FunctionReturnMap {
    functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.return_type.clone(),
            )
        })
        .collect()
}

struct PrivateCallSiteConstraintContext<'a> {
    uses: &'a [UseDecl],
    function_by_path: &'a FunctionAstMap<'a>,
    omitted_private_slots: &'a PrivateSlotMap,
    signatures_by_path: &'a FunctionSignatureMap,
    returns_by_path: &'a FunctionReturnMap,
    functions: &'a mut [FunctionSignature],
    adts: &'a AdtRegistry,
    changed: &'a mut bool,
}

struct PrivateCallSiteExprContext<'a, 'b> {
    current_module: Option<&'b str>,
    caller_key: Option<&'b FunctionKey>,
    bindings: &'b [Binding],
    constraints: &'b mut PrivateCallSiteConstraintContext<'a>,
}

fn collect_private_call_site_constraints(
    function: &Function,
    context: &mut PrivateCallSiteConstraintContext<'_>,
) {
    #[cfg(test)]
    private_inference_counters::record_call_site_scan();

    let current_module = function.module_name.as_deref();
    let caller_key = function
        .name
        .as_ref()
        .map(|name| (function.module_name.clone(), name.clone()));
    let mut bindings = private_function_body_bindings(function, context.signatures_by_path);
    let declared_return = function.return_type.as_deref().map_or_else(
        || {
            caller_key
                .as_ref()
                .and_then(|key| context.signatures_by_path.get(key))
                .map(|signature| signature.return_type.clone())
                .filter(|ty| !type_has_unknown(ty))
        },
        |return_type| Some(parse_type_or_unknown(Some(return_type))),
    );

    for (index, line) in function.body.iter().enumerate() {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                collect_private_call_site_expr_constraints(
                    expr,
                    annotation_type.as_ref(),
                    &mut PrivateCallSiteExprContext {
                        current_module,
                        caller_key: caller_key.as_ref(),
                        bindings: &bindings,
                        constraints: context,
                    },
                );
                let initializer_private_function = annotation_type
                    .is_none()
                    .then(|| {
                        private_same_module_call_target(
                            expr,
                            current_module,
                            context.function_by_path,
                        )
                    })
                    .flatten();
                let ty = annotation_type.unwrap_or_else(|| {
                    infer_private_signature_expr_type(
                        expr,
                        None,
                        current_module,
                        context.uses,
                        &bindings,
                        context.returns_by_path,
                        context.adts,
                    )
                });
                collect_let_pattern_bindings(
                    pattern,
                    &ty,
                    initializer_private_function,
                    &mut bindings,
                );
            }
            BodyLineKind::Expr { expr } => {
                let expected = (index + 1 == function.body.len())
                    .then_some(declared_return.as_ref())
                    .flatten();
                collect_private_call_site_expr_constraints(
                    expr,
                    expected,
                    &mut PrivateCallSiteExprContext {
                        current_module,
                        caller_key: caller_key.as_ref(),
                        bindings: &bindings,
                        constraints: context,
                    },
                );
            }
        }
    }
}

fn collect_private_call_site_expr_constraints(
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    match &expr.kind {
        ExprKind::List(items) => {
            let item_expected = expected.and_then(Type::vec_part);
            for item in items {
                collect_private_call_site_expr_constraints(item, item_expected, context);
            }
        }
        ExprKind::Dict(entries) => {
            let (key_expected, value_expected) = expected
                .and_then(Type::dict_parts)
                .map_or((None, None), |(key, value)| (Some(key), Some(value)));
            for entry in entries {
                collect_private_call_site_expr_constraints(&entry.key, key_expected, context);
                collect_private_call_site_expr_constraints(&entry.value, value_expected, context);
            }
        }
        ExprKind::Record(fields) => {
            for field in fields {
                let field_expected =
                    expected.and_then(|expected| expected.record_field(&field.name));
                collect_private_call_site_expr_constraints(&field.expr, field_expected, context);
            }
        }
        ExprKind::Call { callee, args } => {
            collect_private_call_site_call_constraints(callee, args, expected, context);
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_private_call_site_expr_constraints(arg, None, context);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_private_call_site_expr_constraints(body, expected, context);
            for arg in args {
                collect_private_call_site_expr_constraints(arg, None, context);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_private_call_site_expr_constraints(
                input,
                Some(&Type::named("ByteView", Vec::new())),
                context,
            );
            collect_private_call_site_expr_constraints(
                base,
                Some(&Type::named("ByteOffset", Vec::new())),
                context,
            );
        }
        ExprKind::SchemaEncode { value, .. } => {
            collect_private_call_site_expr_constraints(value, None, context);
        }
        ExprKind::FieldAccess { base, .. }
        | ExprKind::Try(base)
        | ExprKind::Prefix { expr: base, .. } => {
            collect_private_call_site_expr_constraints(base, None, context);
        }
        ExprKind::Match { scrutinee, arms } => {
            let scrutinee_expected = match infer_match_scrutinee_type_from_constructor_patterns(
                arms,
                context.current_module,
                context.constraints.uses,
                context.constraints.adts,
            ) {
                MatchScrutineePatternInference::Inferred(ty) => Some(ty),
                MatchScrutineePatternInference::Uninferred
                | MatchScrutineePatternInference::Ambiguous(_) => None,
            };
            collect_private_call_site_expr_constraints(
                scrutinee,
                scrutinee_expected.as_ref(),
                context,
            );
            for arm in arms {
                collect_private_call_site_expr_constraints(&arm.expr, expected, context);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_private_call_site_expr_constraints(condition, Some(&Type::bool()), context);
            collect_private_call_site_expr_constraints(then_branch, expected, context);
            for branch in else_if_branches {
                collect_private_call_site_expr_constraints(
                    &branch.condition,
                    Some(&Type::bool()),
                    context,
                );
                collect_private_call_site_expr_constraints(&branch.expr, expected, context);
            }
            collect_private_call_site_expr_constraints(else_branch, expected, context);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_private_call_site_expr_constraints(left, expected, context);
            collect_private_call_site_expr_constraints(right, expected, context);
        }
        ExprKind::NamePath(segments) => {
            collect_private_parameter_constraints(segments, expected, context);
            collect_private_function_value_constraints(segments, expected, context);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => {}
    }
}

fn collect_private_parameter_constraints(
    segments: &[String],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let Some(expected) = expected.filter(|ty| !type_has_unknown(ty)) else {
        return;
    };
    let [name] = segments else {
        return;
    };
    let Some(caller_key) = context.caller_key else {
        return;
    };
    let Some((omitted_params, _)) = context.constraints.omitted_private_slots.get(caller_key)
    else {
        return;
    };
    let Some(function) = context.constraints.function_by_path.get(caller_key) else {
        return;
    };
    let Some(index) = function
        .params
        .iter()
        .position(|param| param.name == *name && parameter_annotation_is_omitted(param))
    else {
        return;
    };
    if !omitted_params.get(index).copied().unwrap_or(false) {
        return;
    }
    if function.params[index].is_variadic {
        let Some(item_type) = expected.vec_part().filter(|ty| !type_has_unknown(ty)) else {
            return;
        };
        update_private_signature_variadic(
            context.constraints.functions,
            caller_key,
            item_type.clone(),
            context.constraints.changed,
        );
    } else {
        update_private_signature_param(
            context.constraints.functions,
            caller_key,
            index,
            expected.clone(),
            context.constraints.changed,
        );
    }
}

fn collect_private_call_site_call_constraints(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let Some(target_key) = private_same_module_call_target(
        callee,
        context.current_module,
        context.constraints.function_by_path,
    ) else {
        collect_private_call_site_non_target_call_args(callee, args, expected, context);
        return;
    };

    let is_recursive_edge = context.caller_key == Some(&target_key);
    if !is_recursive_edge
        && let Some((omitted_params, omitted_return)) =
            context.constraints.omitted_private_slots.get(&target_key)
    {
        if let Some(target_params) = context
            .constraints
            .signatures_by_path
            .get(&target_key)
            .map(|signature| signature.params.clone())
        {
            for (index, arg) in args.iter().enumerate() {
                if omitted_params.get(index).copied().unwrap_or(false) {
                    let actual = infer_private_signature_expr_type(
                        arg,
                        None,
                        context.current_module,
                        context.constraints.uses,
                        context.bindings,
                        context.constraints.returns_by_path,
                        context.constraints.adts,
                    );
                    if !type_has_unknown(&actual) {
                        update_private_signature_param(
                            context.constraints.functions,
                            &target_key,
                            index,
                            actual,
                            context.constraints.changed,
                        );
                    }
                }
                let arg_expected = target_params
                    .get(index)
                    .filter(|ty| private_expected_can_constrain(ty));
                collect_private_call_site_expr_constraints(arg, arg_expected, context);
            }
        }

        if *omitted_return
            && let Some(expected) = expected
            && !type_has_unknown(expected)
        {
            update_private_signature_return(
                context.constraints.functions,
                &target_key,
                expected.clone(),
                context.constraints.changed,
            );
        }
    }

    if context
        .constraints
        .omitted_private_slots
        .contains_key(&target_key)
    {
        return;
    }
    let Some(target_params) = context
        .constraints
        .signatures_by_path
        .get(&target_key)
        .map(|signature| signature.params.clone())
    else {
        return;
    };
    for (index, arg) in args.iter().enumerate() {
        let arg_expected = target_params
            .get(index)
            .filter(|ty| private_expected_can_constrain(ty));
        collect_private_call_site_expr_constraints(arg, arg_expected, context);
    }
}

fn collect_private_call_site_non_target_call_args(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let ExprKind::NamePath(segments) = &callee.kind else {
        for arg in args {
            collect_private_call_site_expr_constraints(arg, None, context);
        }
        return;
    };
    let params = private_call_site_non_target_params(segments, args, expected, context);
    for (index, arg) in args.iter().enumerate() {
        let arg_expected = params
            .get(index)
            .filter(|ty| private_expected_can_constrain(ty));
        collect_private_call_site_expr_constraints(arg, arg_expected, context);
    }
}

fn private_expected_can_constrain(ty: &Type) -> bool {
    if !type_has_unknown(ty) {
        return true;
    }
    matches!(
        ty,
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } if !variadic.as_deref().is_some_and(type_has_unknown)
            && (params.iter().any(|param| !type_has_unknown(param))
            || !type_has_unknown(return_type)
            || variadic.as_deref().is_some_and(|ty| !type_has_unknown(ty)))
    )
}

fn private_call_site_non_target_params(
    segments: &[String],
    args: &[Expr],
    expected: Option<&Type>,
    context: &PrivateCallSiteExprContext<'_, '_>,
) -> Vec<Type> {
    if let crate::adt::ConstructorLookup::Found(constructor) = context.constraints.adts.constructor(
        segments,
        context.current_module,
        context.constraints.uses,
    ) {
        return expected
            .and_then(|expected| adt::adt_args(expected, constructor.descriptor))
            .map(|_| {
                constructor
                    .variant
                    .payload_fields
                    .iter()
                    .enumerate()
                    .map(|(index, _)| {
                        expected
                            .and_then(|expected| adt::payload_type(expected, constructor, index))
                            .filter(|ty| !type_has_unknown(ty))
                            .unwrap_or(Type::Unknown)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
    }

    if let Some(signature) = private_call_site_declared_signature(
        segments,
        context.current_module,
        context.constraints.uses,
        context.constraints.signatures_by_path,
    )
    .filter(|signature| {
        context.current_module == Some("std::prelude")
            || signature.module_name.as_deref() != Some("std::prelude")
    }) {
        return signature.params.clone();
    }

    private_prelude_constraint_name(
        segments,
        context.current_module,
        context.constraints.function_by_path,
    )
    .and_then(|name| {
        let input_type = private_prelude_input_arg(args, name).map(|arg| {
            infer_private_signature_expr_type(
                arg,
                None,
                context.current_module,
                context.constraints.uses,
                context.bindings,
                context.constraints.returns_by_path,
                context.constraints.adts,
            )
        });
        let mut params =
            crate::prelude::prelude_signature_with_input(name, expected, input_type.as_ref())
                .map(|(params, _)| params)?;
        if name == "vec_try_map_with" {
            let context_type = args.first().map(|arg| {
                infer_private_signature_expr_type(
                    arg,
                    None,
                    context.current_module,
                    context.constraints.uses,
                    context.bindings,
                    context.constraints.returns_by_path,
                    context.constraints.adts,
                )
            });
            apply_vec_try_map_with_context_param(&mut params, context_type);
        }
        Some(params)
    })
    .unwrap_or_default()
}

fn private_prelude_input_arg<'a>(args: &'a [Expr], helper_name: &str) -> Option<&'a Expr> {
    match helper_name {
        "vec_try_map_with" | "dict_map_with" | "dict_filter_with" | "dict_fold_with"
        | "dict_try_map_with" => args.get(1),
        _ => args.first(),
    }
}

fn collect_private_function_value_constraints(
    segments: &[String],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let expected = expected.filter(|ty| private_expected_can_constrain(ty));
    let Some(Type::Function {
        params,
        variadic,
        return_type,
        ..
    }) = expected
    else {
        return;
    };
    let Some(target_key) = private_function_value_target(segments, context) else {
        return;
    };
    if context.caller_key == Some(&target_key) {
        return;
    }
    let Some(target_function) = context.constraints.function_by_path.get(&target_key) else {
        return;
    };
    let Some((omitted_params, omitted_return)) =
        context.constraints.omitted_private_slots.get(&target_key)
    else {
        return;
    };
    for (index, param) in params.iter().enumerate() {
        if omitted_params.get(index).copied().unwrap_or(false) && !type_has_unknown(param) {
            update_private_signature_param(
                context.constraints.functions,
                &target_key,
                index,
                param.clone(),
                context.constraints.changed,
            );
        }
    }
    if let Some(variadic) = variadic.as_deref().filter(|ty| !type_has_unknown(ty))
        && let Some(index) = target_function
            .params
            .iter()
            .position(|param| param.is_variadic && parameter_annotation_is_omitted(param))
        && omitted_params.get(index).copied().unwrap_or(false)
    {
        update_private_signature_variadic(
            context.constraints.functions,
            &target_key,
            variadic.clone(),
            context.constraints.changed,
        );
    }
    if *omitted_return && !type_has_unknown(return_type) {
        update_private_signature_return(
            context.constraints.functions,
            &target_key,
            return_type.as_ref().clone(),
            context.constraints.changed,
        );
    }
}

fn private_function_value_target(
    segments: &[String],
    context: &PrivateCallSiteExprContext<'_, '_>,
) -> Option<FunctionKey> {
    let [name] = segments else {
        return None;
    };
    if let Some(binding) = context
        .bindings
        .iter()
        .rev()
        .find(|binding| binding.name == *name)
    {
        return binding.private_function_value.clone();
    }
    Some((context.current_module.map(str::to_string), name.clone()))
}

fn private_call_site_declared_signature<'a>(
    segments: &[String],
    current_module: Option<&str>,
    uses: &[UseDecl],
    signatures_by_path: &'a FunctionSignatureMap,
) -> Option<&'a FunctionSignature> {
    match segments {
        [name] => signatures_by_path.get(&(current_module.map(str::to_string), name.clone())),
        [_, .., name] => {
            imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
                .and_then(|use_decl| {
                    signatures_by_path.get(&(Some(use_decl.name.clone()), name.clone()))
                })
                .filter(|signature| signature.visibility == Visibility::Public)
        }
        _ => None,
    }
}

fn parameter_annotation_is_omitted(param: &veln_ast::Param) -> bool {
    param
        .ty
        .as_deref()
        .is_none_or(|annotation| param.is_variadic && annotation.is_empty())
}

fn private_name_path_target(
    segments: &[String],
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
) -> Option<FunctionKey> {
    let [name] = segments else {
        return None;
    };
    let key = (current_module.map(str::to_string), name.clone());
    let function = function_by_path.get(&key)?;
    (function.kind == FunctionKind::Function && function.visibility == Visibility::Private)
        .then_some(key)
}

fn private_same_module_call_target(
    callee: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
) -> Option<FunctionKey> {
    let ExprKind::NamePath(segments) = &callee.kind else {
        return None;
    };
    private_name_path_target(segments, current_module, function_by_path)
}

fn update_private_signature_param(
    functions: &mut [FunctionSignature],
    key: &(Option<String>, String),
    index: usize,
    inferred: Type,
    changed: &mut bool,
) {
    let Some(signature) = functions
        .iter_mut()
        .find(|function| function.module_name == key.0 && function.name == key.1)
    else {
        return;
    };
    let Some(current) = signature.params.get_mut(index) else {
        return;
    };
    if type_has_unknown(current) {
        *current = inferred;
        *changed = true;
    }
}

fn update_private_signature_variadic(
    functions: &mut [FunctionSignature],
    key: &(Option<String>, String),
    inferred: Type,
    changed: &mut bool,
) {
    let Some(signature) = functions
        .iter_mut()
        .find(|function| function.module_name == key.0 && function.name == key.1)
    else {
        return;
    };
    let Some(current) = signature.variadic.as_mut() else {
        return;
    };
    if type_has_unknown(current) {
        *current = inferred;
        *changed = true;
    }
}

fn update_private_signature_return(
    functions: &mut [FunctionSignature],
    key: &(Option<String>, String),
    inferred: Type,
    changed: &mut bool,
) {
    let Some(signature) = functions
        .iter_mut()
        .find(|function| function.module_name == key.0 && function.name == key.1)
    else {
        return;
    };
    if type_has_unknown(&signature.return_type) {
        signature.return_type = inferred;
        *changed = true;
    }
}

fn infer_private_prelude_callback_return_types(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    adts: &AdtRegistry,
) {
    let function_by_path = module
        .functions
        .iter()
        .filter_map(|function| {
            Some((
                (function.module_name.clone(), function.name.clone()?),
                function,
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let mut returns_by_path = functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.return_type.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let initial_omitted_private_returns =
        omitted_private_returns_requiring_prelude_pass(module, functions, &module.uses, adts);
    if initial_omitted_private_returns.is_empty() {
        return;
    }
    let private_references = private_reference_map(
        module,
        &function_by_path,
        &modules_with_private_return_omissions(&initial_omitted_private_returns),
        &initial_omitted_private_returns,
    );
    let contributors = private_prelude_callback_constraint_contributors(
        module,
        &initial_omitted_private_returns,
        &returns_by_path,
        &function_by_path,
        &private_references,
        &module.uses,
        adts,
    );
    if contributors.is_empty() {
        return;
    }

    let mut changed = true;
    while changed {
        changed = false;
        let omitted_private_returns = initial_omitted_private_returns.clone();
        for function in module.functions.iter().filter(|function| {
            function_key(function).is_some_and(|key| contributors.contains(&key))
        }) {
            collect_private_prelude_callback_return_constraints(
                function,
                &module.uses,
                &function_by_path,
                &omitted_private_returns,
                &mut returns_by_path,
                adts,
                &mut changed,
            );
        }
        for function in functions.iter_mut() {
            let key = (function.module_name.clone(), function.name.clone());
            if !omitted_private_returns.contains(&key) {
                continue;
            }
            if let Some(inferred) = returns_by_path.get(&key)
                && inferred != &function.return_type
            {
                function.return_type = inferred.clone();
            }
        }
    }
}

fn collect_private_prelude_callback_return_constraints(
    function: &Function,
    uses: &[UseDecl],
    function_by_path: &BTreeMap<(Option<String>, String), &Function>,
    omitted_private_returns: &BTreeSet<(Option<String>, String)>,
    returns_by_path: &mut BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
    changed: &mut bool,
) {
    #[cfg(test)]
    private_inference_counters::record_prelude_callback_scan();

    let mut bindings = function
        .params
        .iter()
        .map(|param| Binding::new(param.name.clone(), function_body_param_type(param)))
        .collect::<Vec<_>>();
    let declared_return = function
        .return_type
        .as_deref()
        .map(|return_type| parse_type_or_unknown(Some(return_type)));
    for (index, line) in function.body.iter().enumerate() {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                collect_private_prelude_callback_expr_constraints(
                    expr,
                    annotation_type.as_ref(),
                    &mut PrivatePreludeCallbackConstraintContext {
                        current_module: function.module_name.as_deref(),
                        uses,
                        bindings: &bindings,
                        function_by_path,
                        omitted_private_returns,
                        returns_by_path,
                        adts,
                        changed,
                    },
                );
                let initializer_private_function = annotation_type
                    .is_none()
                    .then(|| {
                        private_same_module_call_target(
                            expr,
                            function.module_name.as_deref(),
                            function_by_path,
                        )
                    })
                    .flatten();
                let ty = annotation_type.unwrap_or_else(|| {
                    infer_private_signature_expr_type(
                        expr,
                        None,
                        function.module_name.as_deref(),
                        uses,
                        &bindings,
                        returns_by_path,
                        adts,
                    )
                });
                collect_let_pattern_bindings(
                    pattern,
                    &ty,
                    initializer_private_function,
                    &mut bindings,
                );
            }
            BodyLineKind::Expr { expr } => {
                let expected = (index + 1 == function.body.len())
                    .then_some(declared_return.as_ref())
                    .flatten();
                collect_private_prelude_callback_expr_constraints(
                    expr,
                    expected,
                    &mut PrivatePreludeCallbackConstraintContext {
                        current_module: function.module_name.as_deref(),
                        uses,
                        bindings: &bindings,
                        function_by_path,
                        omitted_private_returns,
                        returns_by_path,
                        adts,
                        changed,
                    },
                );
            }
        }
    }
}

fn private_prelude_callback_constraint_contributors(
    module: &SurfaceModule,
    omitted_private_returns: &BTreeSet<FunctionKey>,
    returns_by_path: &FunctionReturnMap,
    function_by_path: &FunctionAstMap<'_>,
    private_references: &PrivateReferenceMap,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> BTreeSet<FunctionKey> {
    let modules_with_omitted_returns = omitted_private_returns
        .iter()
        .map(|key| key.0.clone())
        .collect::<BTreeSet<_>>();
    module
        .functions
        .iter()
        .filter(|function| modules_with_omitted_returns.contains(&function.module_name))
        .filter_map(|function| {
            let key = function_key(function)?;
            if !omitted_private_returns.contains(&key)
                && !private_references.get(&key).is_some_and(|references| {
                    references
                        .iter()
                        .any(|reference| omitted_private_returns.contains(reference))
                })
            {
                return None;
            }
            private_prelude_callback_function_can_constrain(
                function,
                &key,
                omitted_private_returns,
                returns_by_path,
                function_by_path,
                uses,
                adts,
            )
            .then_some(key)
        })
        .collect()
}

fn private_prelude_callback_function_can_constrain(
    function: &Function,
    key: &FunctionKey,
    omitted_private_returns: &BTreeSet<FunctionKey>,
    returns_by_path: &FunctionReturnMap,
    function_by_path: &FunctionAstMap<'_>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> bool {
    if omitted_private_returns.contains(key)
        && returns_by_path.get(key).is_some_and(|return_type| {
            private_tail_can_use_expected(function, return_type, uses, adts)
        })
    {
        return true;
    }

    #[cfg(test)]
    private_inference_counters::record_prelude_callback_discovery_scan();

    let mut bindings = function
        .params
        .iter()
        .map(|param| Binding::new(param.name.clone(), function_body_param_type(param)))
        .collect::<Vec<_>>();
    let declared_return = function
        .return_type
        .as_deref()
        .map(|return_type| parse_type_or_unknown(Some(return_type)));
    for (index, line) in function.body.iter().enumerate() {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                let reference_context = PrivatePreludeCallbackReferenceContext {
                    current_module: function.module_name.as_deref(),
                    uses,
                    bindings: &bindings,
                    omitted_private_returns,
                    returns_by_path,
                    function_by_path,
                    adts,
                };
                if private_prelude_callback_expr_references_slot(
                    expr,
                    annotation_type.as_ref(),
                    &reference_context,
                ) {
                    return true;
                }
                let initializer_private_function = annotation_type
                    .is_none()
                    .then(|| {
                        private_same_module_call_target(
                            expr,
                            function.module_name.as_deref(),
                            function_by_path,
                        )
                    })
                    .flatten();
                let ty = annotation_type.unwrap_or_else(|| {
                    infer_private_signature_expr_type(
                        expr,
                        None,
                        function.module_name.as_deref(),
                        uses,
                        &bindings,
                        returns_by_path,
                        adts,
                    )
                });
                collect_let_pattern_bindings(
                    pattern,
                    &ty,
                    initializer_private_function,
                    &mut bindings,
                );
            }
            BodyLineKind::Expr { expr } => {
                let expected = (index + 1 == function.body.len())
                    .then_some(declared_return.as_ref())
                    .flatten();
                let reference_context = PrivatePreludeCallbackReferenceContext {
                    current_module: function.module_name.as_deref(),
                    uses,
                    bindings: &bindings,
                    omitted_private_returns,
                    returns_by_path,
                    function_by_path,
                    adts,
                };
                if private_prelude_callback_expr_references_slot(expr, expected, &reference_context)
                {
                    return true;
                }
            }
        }
    }
    false
}

fn private_prelude_callback_expr_references_slot(
    expr: &Expr,
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    if let ExprKind::NamePath(segments) = &expr.kind
        && expected.is_some_and(|expected| {
            private_callback_return_constraint_can_update(segments, expected, context)
        })
    {
        return true;
    }
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            let direct_reference =
                private_prelude_callback_call_references_slot(callee, args, expected, context);
            direct_reference
                || !matches!(callee.kind, ExprKind::NamePath(_))
                    && private_prelude_callback_expr_references_slot(callee, None, context)
                || args
                    .iter()
                    .any(|arg| private_prelude_callback_expr_references_slot(arg, None, context))
        }
        ExprKind::List(_) | ExprKind::Dict(_) | ExprKind::Record(_) => {
            private_prelude_callback_collection_references_slot(expr, expected, context)
        }
        ExprKind::Perform { .. }
        | ExprKind::Handle { .. }
        | ExprKind::SchemaDecode { .. }
        | ExprKind::SchemaEncode { .. }
        | ExprKind::FieldAccess { .. }
        | ExprKind::Try(_)
        | ExprKind::Prefix { .. } => {
            private_prelude_callback_wrapped_expr_references_slot(expr, expected, context)
        }
        ExprKind::Match { .. } | ExprKind::If { .. } | ExprKind::Binary { .. } => {
            private_prelude_callback_control_flow_references_slot(expr, expected, context)
        }
        ExprKind::NamePath(_)
        | ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => false,
    }
}

fn private_prelude_callback_collection_references_slot(
    expr: &Expr,
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    match &expr.kind {
        ExprKind::List(items) => items.iter().any(|item| {
            let item_expected = expected.and_then(Type::vec_part);
            private_prelude_callback_expr_references_slot(item, item_expected, context)
        }),
        ExprKind::Dict(entries) => entries.iter().any(|entry| {
            let (key_expected, value_expected) = expected
                .and_then(Type::dict_parts)
                .map_or((None, None), |(key, value)| (Some(key), Some(value)));
            private_prelude_callback_expr_references_slot(&entry.key, key_expected, context)
                || private_prelude_callback_expr_references_slot(
                    &entry.value,
                    value_expected,
                    context,
                )
        }),
        ExprKind::Record(fields) => fields.iter().any(|field| {
            let field_expected = expected.and_then(|expected| expected.record_field(&field.name));
            private_prelude_callback_expr_references_slot(&field.expr, field_expected, context)
        }),
        _ => false,
    }
}

fn private_prelude_callback_wrapped_expr_references_slot(
    expr: &Expr,
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    match &expr.kind {
        ExprKind::Perform { args, .. } => args
            .iter()
            .any(|arg| private_prelude_callback_expr_references_slot(arg, None, context)),
        ExprKind::Handle { body, args, .. } => {
            private_prelude_callback_expr_references_slot(body, expected, context)
                || args
                    .iter()
                    .any(|arg| private_prelude_callback_expr_references_slot(arg, None, context))
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            private_prelude_callback_expr_references_slot(
                input,
                Some(&Type::named("ByteView", Vec::new())),
                context,
            ) || private_prelude_callback_expr_references_slot(
                base,
                Some(&Type::named("ByteOffset", Vec::new())),
                context,
            )
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::FieldAccess { base: value, .. }
        | ExprKind::Try(value)
        | ExprKind::Prefix { expr: value, .. } => {
            private_prelude_callback_expr_references_slot(value, None, context)
        }
        _ => false,
    }
}

fn private_prelude_callback_control_flow_references_slot(
    expr: &Expr,
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    match &expr.kind {
        ExprKind::Match { scrutinee, arms } => {
            private_prelude_callback_expr_references_slot(scrutinee, None, context)
                || arms.iter().any(|arm| {
                    private_prelude_callback_expr_references_slot(&arm.expr, expected, context)
                })
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            private_prelude_callback_expr_references_slot(condition, Some(&Type::bool()), context)
                || private_prelude_callback_expr_references_slot(then_branch, expected, context)
                || else_if_branches.iter().any(|branch| {
                    private_prelude_callback_expr_references_slot(
                        &branch.condition,
                        Some(&Type::bool()),
                        context,
                    ) || private_prelude_callback_expr_references_slot(
                        &branch.expr,
                        expected,
                        context,
                    )
                })
                || private_prelude_callback_expr_references_slot(else_branch, expected, context)
        }
        ExprKind::Binary { left, right, .. } => {
            private_prelude_callback_expr_references_slot(left, expected, context)
                || private_prelude_callback_expr_references_slot(right, expected, context)
        }
        _ => false,
    }
}

fn private_prelude_callback_call_references_slot(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    let ExprKind::NamePath(segments) = &callee.kind else {
        return false;
    };
    let Some(name) =
        private_prelude_constraint_name(segments, context.current_module, context.function_by_path)
    else {
        return false;
    };
    let input_type = private_prelude_input_arg(args, name).map(|arg| {
        infer_private_signature_expr_type(
            arg,
            None,
            context.current_module,
            context.uses,
            context.bindings,
            context.returns_by_path,
            context.adts,
        )
    });
    let Some((mut params, _)) =
        crate::prelude::prelude_signature_with_input(name, expected, input_type.as_ref())
    else {
        return false;
    };
    if name == "vec_try_map_with" {
        let context_type = args.first().map(|arg| {
            infer_private_signature_expr_type(
                arg,
                None,
                context.current_module,
                context.uses,
                context.bindings,
                context.returns_by_path,
                context.adts,
            )
        });
        apply_vec_try_map_with_context_param(&mut params, context_type);
    }
    args.iter()
        .zip(params.iter())
        .any(|(arg, param)| private_prelude_callback_arg_references_slot(arg, param, context))
}

struct PrivatePreludeCallbackReferenceContext<'a> {
    current_module: Option<&'a str>,
    uses: &'a [UseDecl],
    bindings: &'a [Binding],
    omitted_private_returns: &'a BTreeSet<FunctionKey>,
    returns_by_path: &'a FunctionReturnMap,
    function_by_path: &'a FunctionAstMap<'a>,
    adts: &'a AdtRegistry,
}

fn private_prelude_callback_arg_references_slot(
    expr: &Expr,
    expected: &Type,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    match &expr.kind {
        ExprKind::NamePath(segments) => {
            private_callback_return_constraint_can_update(segments, expected, context)
        }
        _ => private_prelude_callback_expr_references_slot(expr, Some(expected), context),
    }
}

fn private_callback_return_constraint_can_update(
    segments: &[String],
    expected_callback: &Type,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    let Type::Function { return_type, .. } = expected_callback else {
        return false;
    };
    if type_has_unknown(return_type) {
        return false;
    }
    let [name] = segments else {
        return false;
    };
    let key = context
        .bindings
        .iter()
        .rev()
        .find(|binding| binding.name == *name)
        .and_then(|binding| binding.private_function_value.clone())
        .unwrap_or_else(|| (context.current_module.map(str::to_string), name.clone()));
    if !context.omitted_private_returns.contains(&key) {
        return false;
    }
    let Some(function) = context.function_by_path.get(&key) else {
        return false;
    };
    if !private_tail_can_use_expected(function, return_type, context.uses, context.adts) {
        return false;
    }
    context.returns_by_path.get(&key) != Some(return_type)
}

struct PrivatePreludeCallbackConstraintContext<'a> {
    current_module: Option<&'a str>,
    uses: &'a [UseDecl],
    bindings: &'a [Binding],
    function_by_path: &'a BTreeMap<(Option<String>, String), &'a Function>,
    omitted_private_returns: &'a BTreeSet<(Option<String>, String)>,
    returns_by_path: &'a mut BTreeMap<(Option<String>, String), Type>,
    adts: &'a AdtRegistry,
    changed: &'a mut bool,
}

fn collect_private_prelude_callback_expr_constraints(
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    match &expr.kind {
        ExprKind::List(items) => {
            let item_expected = expected.and_then(Type::vec_part);
            for item in items {
                collect_private_prelude_callback_expr_constraints(item, item_expected, context);
            }
        }
        ExprKind::Dict(entries) => {
            let (key_expected, value_expected) = expected
                .and_then(Type::dict_parts)
                .map_or((None, None), |(key, value)| (Some(key), Some(value)));
            for entry in entries {
                collect_private_prelude_callback_expr_constraints(
                    &entry.key,
                    key_expected,
                    context,
                );
                collect_private_prelude_callback_expr_constraints(
                    &entry.value,
                    value_expected,
                    context,
                );
            }
        }
        ExprKind::Record(fields) => {
            for field in fields {
                let field_expected =
                    expected.and_then(|expected| expected.record_field(&field.name));
                collect_private_prelude_callback_expr_constraints(
                    &field.expr,
                    field_expected,
                    context,
                );
            }
        }
        ExprKind::Call { callee, args } => {
            collect_private_prelude_callback_call_constraints(callee, args, expected, context);
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_private_prelude_callback_expr_constraints(arg, None, context);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_private_prelude_callback_expr_constraints(body, expected, context);
            for arg in args {
                collect_private_prelude_callback_expr_constraints(arg, None, context);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_private_prelude_callback_expr_constraints(
                input,
                Some(&Type::named("ByteView", Vec::new())),
                context,
            );
            collect_private_prelude_callback_expr_constraints(
                base,
                Some(&Type::named("ByteOffset", Vec::new())),
                context,
            );
        }
        ExprKind::SchemaEncode { value, .. } => {
            collect_private_prelude_callback_expr_constraints(value, None, context);
        }
        ExprKind::FieldAccess { base, .. }
        | ExprKind::Try(base)
        | ExprKind::Prefix { expr: base, .. } => {
            collect_private_prelude_callback_expr_constraints(base, None, context);
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_private_prelude_callback_expr_constraints(scrutinee, None, context);
            for arm in arms {
                collect_private_prelude_callback_expr_constraints(&arm.expr, expected, context);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_private_prelude_callback_expr_constraints(
                condition,
                Some(&Type::bool()),
                context,
            );
            collect_private_prelude_callback_expr_constraints(then_branch, expected, context);
            for branch in else_if_branches {
                collect_private_prelude_callback_expr_constraints(
                    &branch.condition,
                    Some(&Type::bool()),
                    context,
                );
                collect_private_prelude_callback_expr_constraints(&branch.expr, expected, context);
            }
            collect_private_prelude_callback_expr_constraints(else_branch, expected, context);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_private_prelude_callback_expr_constraints(left, expected, context);
            collect_private_prelude_callback_expr_constraints(right, expected, context);
        }
        ExprKind::NamePath(segments) => {
            if let Some(expected) = expected {
                collect_private_callback_return_constraint_for_segments(
                    segments, expected, context,
                );
            }
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => {}
    }
}

fn collect_private_prelude_callback_call_constraints(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    let ExprKind::NamePath(segments) = &callee.kind else {
        return;
    };
    let Some(name) =
        private_prelude_constraint_name(segments, context.current_module, context.function_by_path)
    else {
        return;
    };
    let input_type = private_prelude_input_arg(args, name).map(|arg| {
        infer_private_signature_expr_type(
            arg,
            None,
            context.current_module,
            context.uses,
            context.bindings,
            context.returns_by_path,
            context.adts,
        )
    });
    let Some((mut params, _)) =
        crate::prelude::prelude_signature_with_input(name, expected, input_type.as_ref())
    else {
        return;
    };
    if name == "vec_try_map_with" {
        let context_type = args.first().map(|arg| {
            infer_private_signature_expr_type(
                arg,
                None,
                context.current_module,
                context.uses,
                context.bindings,
                context.returns_by_path,
                context.adts,
            )
        });
        apply_vec_try_map_with_context_param(&mut params, context_type);
    }
    for (arg, param) in args.iter().zip(params.iter()) {
        collect_private_callback_return_constraint(arg, param, context);
        collect_private_prelude_callback_expr_constraints(arg, Some(param), context);
    }
}

fn apply_vec_try_map_with_context_param(params: &mut [Type], context_type: Option<Type>) {
    let Some(context_type) = context_type else {
        return;
    };
    if let Some(param) = params.first_mut() {
        *param = context_type.clone();
    }
    let Some(Type::Function {
        params: callback_params,
        ..
    }) = params.get_mut(2)
    else {
        return;
    };
    if let Some(callback_context) = callback_params.first_mut() {
        *callback_context = context_type;
    }
}

fn private_prelude_constraint_name<'a>(
    segments: &'a [String],
    current_module: Option<&str>,
    function_by_path: &BTreeMap<(Option<String>, String), &Function>,
) -> Option<&'a str> {
    match segments {
        [name]
            if !function_by_path
                .contains_key(&(current_module.map(str::to_string), name.clone())) =>
        {
            Some(name)
        }
        [module, name] if module == "prelude" || module == "prelude_builtin" => Some(name),
        _ => None,
    }
}

fn collect_private_callback_return_constraint(
    arg: &Expr,
    expected_callback: &Type,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    let Type::Function { return_type, .. } = expected_callback else {
        return;
    };
    if type_has_unknown(return_type) {
        return;
    }
    let ExprKind::NamePath(segments) = &arg.kind else {
        return;
    };
    collect_private_callback_return_constraint_for_segments(segments, expected_callback, context);
}

fn collect_private_callback_return_constraint_for_segments(
    segments: &[String],
    expected_callback: &Type,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    let Type::Function { return_type, .. } = expected_callback else {
        return;
    };
    if type_has_unknown(return_type) {
        return;
    }
    let [name] = segments else {
        return;
    };
    let key = context
        .bindings
        .iter()
        .rev()
        .find(|binding| binding.name == *name)
        .and_then(|binding| binding.private_function_value.clone())
        .unwrap_or_else(|| (context.current_module.map(str::to_string), name.clone()));
    if !context.omitted_private_returns.contains(&key) {
        return;
    }
    let Some(function) = context.function_by_path.get(&key) else {
        return;
    };
    if !private_tail_can_use_expected(function, return_type, context.uses, context.adts) {
        return;
    }
    if context.returns_by_path.get(&key) == Some(return_type) {
        return;
    }
    context
        .returns_by_path
        .insert(key, return_type.as_ref().clone());
    *context.changed = true;
}

fn private_tail_can_use_expected(
    function: &Function,
    expected: &Type,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> bool {
    let Some(BodyLineKind::Expr { expr }) = function.body.last().map(|line| &line.kind) else {
        return false;
    };
    tail_expr_can_use_expected(expr, expected, function.module_name.as_deref(), uses, adts)
}

fn tail_expr_can_use_expected(
    expr: &Expr,
    expected: &Type,
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> bool {
    match &expr.kind {
        ExprKind::List(_) => expected.vec_part().is_some(),
        ExprKind::Dict(_) => expected.dict_parts().is_some(),
        ExprKind::Record(fields) => {
            if fields.is_empty() && expected.dict_parts().is_some() {
                return true;
            }
            !fields.is_empty()
                && fields
                    .iter()
                    .all(|field| expected.record_field(&field.name).is_some())
        }
        ExprKind::NamePath(segments) => {
            matches!(
                adts.nullary_constructor(segments, current_module, uses),
                crate::adt::ConstructorLookup::Found(constructor)
                    if adt::adt_args(expected, constructor.descriptor).is_some()
            )
        }
        ExprKind::Call { callee, .. } => {
            let ExprKind::NamePath(segments) = &callee.kind else {
                return false;
            };
            matches!(
                adts.constructor(segments, current_module, uses),
                crate::adt::ConstructorLookup::Found(constructor)
                    if adt::adt_args(expected, constructor.descriptor).is_some()
            )
        }
        ExprKind::Match { arms, .. } => arms
            .iter()
            .all(|arm| tail_expr_can_use_expected(&arm.expr, expected, current_module, uses, adts)),
        ExprKind::If {
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => std::iter::once(then_branch.as_ref())
            .chain(else_if_branches.iter().map(|branch| &branch.expr))
            .chain(std::iter::once(else_branch.as_ref()))
            .all(|branch| tail_expr_can_use_expected(branch, expected, current_module, uses, adts)),
        _ => false,
    }
}

fn infer_private_function_tail_type(
    function: &veln_ast::Function,
    uses: &[UseDecl],
    signatures_by_path: &BTreeMap<(Option<String>, String), FunctionSignature>,
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    #[cfg(test)]
    private_inference_counters::record_body_return_scan();

    let mut bindings = private_function_body_bindings(function, signatures_by_path);
    let mut tail = Type::unit();
    for line in &function.body {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                let ty = annotation_type.unwrap_or_else(|| {
                    infer_private_signature_expr_type(
                        expr,
                        None,
                        function.module_name.as_deref(),
                        uses,
                        &bindings,
                        returns_by_path,
                        adts,
                    )
                });
                collect_pattern_bindings(pattern, &ty, &mut bindings);
            }
            BodyLineKind::Expr { expr } => {
                tail = infer_private_signature_expr_type(
                    expr,
                    None,
                    function.module_name.as_deref(),
                    uses,
                    &bindings,
                    returns_by_path,
                    adts,
                );
            }
        }
    }
    tail
}

fn private_function_body_bindings(
    function: &veln_ast::Function,
    signatures_by_path: &BTreeMap<(Option<String>, String), FunctionSignature>,
) -> Vec<Binding> {
    let signature = function
        .name
        .as_ref()
        .and_then(|name| signatures_by_path.get(&(function.module_name.clone(), name.clone())));
    function
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| {
            let ty = if param.is_variadic {
                signature
                    .and_then(|signature| signature.variadic.clone())
                    .map(|ty| Type::named("List", vec![ty]))
                    .unwrap_or_else(|| function_body_param_type(param))
            } else {
                signature
                    .and_then(|signature| signature.params.get(index).cloned())
                    .unwrap_or_else(|| function_body_param_type(param))
            };
            Binding::new(param.name.clone(), ty)
        })
        .collect()
}

fn infer_private_signature_expr_type(
    expr: &Expr,
    expected: Option<&Type>,
    current_module: Option<&str>,
    uses: &[UseDecl],
    bindings: &[Binding],
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    let context = PrivateSignatureInferContext {
        current_module,
        uses,
        bindings,
        returns_by_path,
        adts,
    };
    match &expr.kind {
        ExprKind::Missing | ExprKind::Hole { .. } | ExprKind::TypeApply { .. } => Type::Unknown,
        ExprKind::StringLiteral(_) => Type::string(),
        ExprKind::IntLiteral(_) => Type::int(),
        ExprKind::FloatLiteral(_) => Type::float(),
        ExprKind::BoolLiteral(_) => Type::bool(),
        ExprKind::Unit => Type::unit(),
        ExprKind::NamePath(segments) => infer_private_signature_name_type(
            segments,
            expected,
            current_module,
            uses,
            bindings,
            returns_by_path,
            adts,
        ),
        ExprKind::List(items) => infer_private_list_type(items, expected, &context),
        ExprKind::Dict(entries) => infer_private_dict_type(entries, expected, &context),
        ExprKind::Record(fields) => infer_private_record_type(fields, expected, &context),
        ExprKind::Call { callee, args } => {
            infer_private_signature_call_type(callee, args, expected, &context)
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                context.infer(arg, None);
            }
            Type::Unknown
        }
        ExprKind::Handle { body, args, .. } => {
            for arg in args {
                context.infer(arg, None);
            }
            context.infer(body, expected)
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            context.infer(input, Some(&Type::named("ByteView", Vec::new())));
            context.infer(base, Some(&Type::named("ByteOffset", Vec::new())));
            Type::Unknown
        }
        ExprKind::SchemaEncode { value, .. } => {
            context.infer(value, None);
            Type::Unknown
        }
        ExprKind::FieldAccess { base, field, .. } => context
            .infer(base, None)
            .record_field(field)
            .cloned()
            .unwrap_or(Type::Unknown),
        ExprKind::Try(inner) => expected.cloned().unwrap_or_else(|| {
            let inner_type = context.infer(inner, None);
            adt::result_parts(&inner_type).map_or(Type::Unknown, |(value, _)| value.clone())
        }),
        ExprKind::Match { scrutinee, arms } => {
            infer_private_match_type(scrutinee, arms, expected, &context)
        }
        ExprKind::If {
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => infer_private_if_result_type(
            then_branch,
            else_if_branches,
            else_branch,
            expected,
            &context,
        ),
        ExprKind::Prefix { expr, .. } => {
            context.infer(expr, expected);
            Type::Unknown
        }
        ExprKind::Binary { op, left, right } => {
            infer_private_binary_type(*op, left, right, expected, &context)
        }
    }
}

fn infer_private_list_type(
    items: &[Expr],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    let mut item_type = expected
        .and_then(Type::vec_part)
        .cloned()
        .unwrap_or(Type::Unknown);
    for item in items {
        let actual = context.infer(item, item_type_unknown_as_none(&item_type));
        if item_type == Type::Unknown {
            item_type = actual;
        }
    }
    Type::vec(item_type)
}

fn infer_private_dict_type(
    entries: &[DictEntry],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    let (mut key_type, mut value_type) = expected
        .and_then(Type::dict_parts)
        .map_or((Type::Unknown, Type::Unknown), |(key, value)| {
            (key.clone(), value.clone())
        });
    for entry in entries {
        let key_actual = context.infer(&entry.key, item_type_unknown_as_none(&key_type));
        if key_type == Type::Unknown {
            key_type = key_actual;
        }
        let value_actual = context.infer(&entry.value, item_type_unknown_as_none(&value_type));
        if value_type == Type::Unknown {
            value_type = value_actual;
        }
    }
    Type::dict(key_type, value_type)
}

fn infer_private_record_type(
    fields: &[RecordField],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    if fields.is_empty()
        && let Some(expected) = expected
        && expected.dict_parts().is_some()
    {
        return expected.clone();
    }
    Type::Record(
        fields
            .iter()
            .map(|field| {
                let field_expected =
                    expected.and_then(|expected| expected.record_field(&field.name));
                (
                    field.name.clone(),
                    context.infer(&field.expr, field_expected),
                )
            })
            .collect(),
    )
}

fn infer_private_match_type(
    scrutinee: &Expr,
    arms: &[MatchArm],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    let scrutinee_expected = match infer_match_scrutinee_type_from_constructor_patterns(
        arms,
        context.current_module,
        context.uses,
        context.adts,
    ) {
        MatchScrutineePatternInference::Inferred(ty) => Some(ty),
        MatchScrutineePatternInference::Uninferred
        | MatchScrutineePatternInference::Ambiguous(_) => None,
    };
    context.infer(scrutinee, scrutinee_expected.as_ref());
    let mut result = expected.cloned().unwrap_or(Type::Unknown);
    for arm in arms {
        let actual = context.infer(&arm.expr, item_type_unknown_as_none(&result));
        if result == Type::Unknown {
            result = actual;
        }
    }
    result
}

fn infer_private_if_result_type(
    then_branch: &Expr,
    else_if_branches: &[IfBranch],
    else_branch: &Expr,
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    let mut result = expected.cloned().unwrap_or(Type::Unknown);
    for branch_expr in std::iter::once(then_branch)
        .chain(else_if_branches.iter().map(|branch| &branch.expr))
        .chain(std::iter::once(else_branch))
    {
        let actual = context.infer(branch_expr, item_type_unknown_as_none(&result));
        if result == Type::Unknown {
            result = actual;
        }
    }
    result
}

fn infer_private_binary_type(
    op: veln_ast::BinaryOp,
    left: &Expr,
    right: &Expr,
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    match op {
        veln_ast::BinaryOp::Equal
        | veln_ast::BinaryOp::NotEqual
        | veln_ast::BinaryOp::Less
        | veln_ast::BinaryOp::LessEqual
        | veln_ast::BinaryOp::Greater
        | veln_ast::BinaryOp::GreaterEqual
        | veln_ast::BinaryOp::Or
        | veln_ast::BinaryOp::And => Type::bool(),
        veln_ast::BinaryOp::BitwiseOr
        | veln_ast::BinaryOp::BitwiseXor
        | veln_ast::BinaryOp::BitwiseAnd
        | veln_ast::BinaryOp::ShiftLeft
        | veln_ast::BinaryOp::ShiftRight
        | veln_ast::BinaryOp::ShiftRightLogical => Type::int(),
        veln_ast::BinaryOp::Add
        | veln_ast::BinaryOp::Subtract
        | veln_ast::BinaryOp::Multiply
        | veln_ast::BinaryOp::Divide => {
            let left = context.infer(left, expected);
            let right = context.infer(right, expected);
            if left == Type::float() || right == Type::float() {
                Type::float()
            } else {
                Type::int()
            }
        }
        veln_ast::BinaryOp::PipeGreater => Type::Unknown,
    }
}

fn item_type_unknown_as_none(ty: &Type) -> Option<&Type> {
    (ty != &Type::Unknown).then_some(ty)
}

pub(crate) fn infer_match_scrutinee_type_from_constructor_patterns(
    arms: &[MatchArm],
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> MatchScrutineePatternInference {
    let mut inferred: Option<(crate::adt::AdtConstructor<'_>, Vec<Type>)> = None;

    for arm in arms {
        let PatternKind::Constructor { name, args } = &arm.pattern.kind else {
            continue;
        };
        let candidates = adts.constructor_candidates(name, current_module, uses);
        if candidates.is_empty() {
            continue;
        }
        let descriptor_names = unique_constructor_descriptor_names(&candidates);
        if descriptor_names.len() != 1 {
            return MatchScrutineePatternInference::Ambiguous(descriptor_names);
        }
        let constructor = candidates[0];
        if let Some((previous, _)) = &inferred {
            if !same_constructor_descriptor(previous, &constructor) {
                let mut names = unique_constructor_descriptor_names(&[*previous, constructor]);
                names.sort();
                return MatchScrutineePatternInference::Ambiguous(names);
            }
        } else {
            inferred = Some((
                constructor,
                vec![Type::Unknown; constructor.descriptor.type_parameters.len()],
            ));
        }
        let Some((_, type_args)) = &mut inferred else {
            continue;
        };
        for (index, pattern) in args.iter().enumerate() {
            let Some(pattern_type) =
                infer_pattern_type_from_constructor_patterns(pattern, current_module, uses, adts)
            else {
                continue;
            };
            adt::merge_type_args_from_payload(type_args, constructor, index, &pattern_type);
        }
    }

    match inferred {
        Some((constructor, type_args)) => MatchScrutineePatternInference::Inferred(
            adt::constructed_type_from_args(constructor, &type_args),
        ),
        None => MatchScrutineePatternInference::Uninferred,
    }
}

fn infer_pattern_type_from_constructor_patterns(
    pattern: &Pattern,
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> Option<Type> {
    match &pattern.kind {
        PatternKind::StringLiteral(_) => Some(Type::string()),
        PatternKind::IntLiteral(_) => Some(Type::int()),
        PatternKind::FloatLiteral(_) => Some(Type::float()),
        PatternKind::BoolLiteral(_) => Some(Type::bool()),
        PatternKind::Unit => Some(Type::unit()),
        PatternKind::Record(fields) => Some(Type::Record(
            fields
                .iter()
                .map(|field| {
                    (
                        field.name.clone(),
                        infer_pattern_type_from_constructor_patterns(
                            &field.pattern,
                            current_module,
                            uses,
                            adts,
                        )
                        .unwrap_or(Type::Unknown),
                    )
                })
                .collect(),
        )),
        PatternKind::Constructor { name, args } => {
            let candidates = adts.constructor_candidates(name, current_module, uses);
            let [constructor] = candidates.as_slice() else {
                return None;
            };
            let mut type_args = vec![Type::Unknown; constructor.descriptor.type_parameters.len()];
            for (index, pattern) in args.iter().enumerate() {
                let Some(pattern_type) = infer_pattern_type_from_constructor_patterns(
                    pattern,
                    current_module,
                    uses,
                    adts,
                ) else {
                    continue;
                };
                adt::merge_type_args_from_payload(
                    &mut type_args,
                    *constructor,
                    index,
                    &pattern_type,
                );
            }
            Some(adt::constructed_type_from_args(*constructor, &type_args))
        }
        PatternKind::Wildcard | PatternKind::Binding(_) => None,
    }
}

fn unique_constructor_descriptor_names(
    constructors: &[crate::adt::AdtConstructor<'_>],
) -> Vec<String> {
    let mut names = Vec::new();
    for constructor in constructors {
        let name = constructor.descriptor.diagnostic_name.clone();
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

fn same_constructor_descriptor(
    left: &crate::adt::AdtConstructor<'_>,
    right: &crate::adt::AdtConstructor<'_>,
) -> bool {
    left.descriptor.type_name == right.descriptor.type_name
        && left.descriptor.module_name == right.descriptor.module_name
        && left.descriptor.type_parameters.len() == right.descriptor.type_parameters.len()
}

fn type_has_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::Named { args, .. } => args.iter().any(type_has_unknown),
        Type::Record(fields) => fields.iter().any(|(_, ty)| type_has_unknown(ty)),
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            params.iter().any(type_has_unknown)
                || variadic.as_deref().is_some_and(type_has_unknown)
                || type_has_unknown(return_type)
        }
    }
}

fn infer_private_signature_name_type(
    segments: &[String],
    expected: Option<&Type>,
    current_module: Option<&str>,
    uses: &[UseDecl],
    bindings: &[Binding],
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    if let crate::adt::ConstructorLookup::Found(constructor) =
        adts.nullary_constructor(segments, current_module, uses)
    {
        return expected
            .and_then(|expected| {
                adt::adt_args(expected, constructor.descriptor).map(|_| expected.clone())
            })
            .unwrap_or_else(|| adt::constructed_type(constructor, &[]));
    }
    match segments {
        [name] => bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)
            .map(|binding| binding.ty.clone())
            .or_else(|| {
                returns_by_path
                    .get(&(current_module.map(str::to_string), name.clone()))
                    .cloned()
            })
            .unwrap_or(Type::Unknown),
        [_, .., name] => {
            imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
                .and_then(|use_decl| {
                    returns_by_path
                        .get(&(Some(use_decl.name.clone()), name.clone()))
                        .cloned()
                })
                .unwrap_or(Type::Unknown)
        }
        _ => Type::Unknown,
    }
}

struct PrivateSignatureInferContext<'a> {
    current_module: Option<&'a str>,
    uses: &'a [UseDecl],
    bindings: &'a [Binding],
    returns_by_path: &'a BTreeMap<(Option<String>, String), Type>,
    adts: &'a AdtRegistry,
}

impl PrivateSignatureInferContext<'_> {
    fn infer(&self, expr: &Expr, expected: Option<&Type>) -> Type {
        infer_private_signature_expr_type(
            expr,
            expected,
            self.current_module,
            self.uses,
            self.bindings,
            self.returns_by_path,
            self.adts,
        )
    }
}

fn infer_private_signature_call_type(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    if let ExprKind::NamePath(segments) = &callee.kind {
        if let crate::adt::ConstructorLookup::Found(constructor) =
            context
                .adts
                .constructor(segments, context.current_module, context.uses)
        {
            let actual_args = args
                .iter()
                .map(|arg| context.infer(arg, None))
                .collect::<Vec<_>>();
            if expected
                .and_then(|expected| adt::adt_args(expected, constructor.descriptor))
                .is_some()
            {
                return expected.cloned().unwrap_or(Type::Unknown);
            }
            return adt::constructed_type(constructor, &actual_args);
        }
        if let Some(name) = segments.last() {
            if let Some(return_type) = match segments.as_slice() {
                [name] => context
                    .returns_by_path
                    .get(&(context.current_module.map(str::to_string), name.clone())),
                [_, .., name] => imported_use_for_path(
                    context.uses,
                    &segments[..segments.len() - 1],
                    context.current_module,
                )
                .and_then(|use_decl| {
                    context
                        .returns_by_path
                        .get(&(Some(use_decl.name.clone()), name.clone()))
                }),
                _ => None,
            } {
                return return_type.clone();
            }
            if let Some((params, return_type)) = crate::prelude::prelude_signature(name, expected) {
                for (arg, param) in args.iter().zip(params.iter()) {
                    context.infer(arg, Some(param));
                }
                return return_type;
            }
        }
    }
    Type::Unknown
}

pub(crate) fn function_body_param_type(param: &veln_ast::Param) -> Type {
    let ty = parse_type_or_unknown(param.ty.as_deref());
    if param.is_variadic {
        Type::named("List", vec![ty])
    } else {
        ty
    }
}

trait SymbolVisibility {
    fn visibility(&self) -> Visibility;
}

impl SymbolVisibility for FunctionSignature {
    fn visibility(&self) -> Visibility {
        self.visibility
    }
}

impl SymbolVisibility for NamedSymbol {
    fn visibility(&self) -> Visibility {
        self.visibility
    }
}

impl SchemaSymbolTable {
    fn from_module(module: &SurfaceModule) -> Self {
        let schemas = module
            .schemas
            .iter()
            .filter_map(|schema| {
                Some(SchemaSymbol {
                    name: schema.name.clone()?,
                    module_name: schema.module_name.clone(),
                    visibility: schema.visibility,
                    span: schema.span.clone(),
                    unsupported_format_neutral_encode_field:
                        format_neutral_schema_first_unsupported_encode_field(module, schema),
                })
            })
            .collect();
        let aliases = module
            .aliases
            .iter()
            .filter(|alias| alias.kind == PublicAliasKind::Schema)
            .filter_map(|alias| {
                Some(SchemaAliasSymbol {
                    name: alias.name.clone()?,
                    module_name: alias.module_name.clone(),
                    target: alias.target.clone(),
                })
            })
            .collect();
        Self { schemas, aliases }
    }

    fn private_schema(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
        companion_access_targets: &BTreeMap<String, String>,
    ) -> bool {
        self.schema_path(
            segments,
            current_module,
            uses,
            true,
            companion_access_targets,
            &mut Vec::new(),
        ) == SchemaPathLookup::Private
    }

    fn schema_alias_target(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<SchemaAliasTarget> {
        match segments {
            [name] => self.schema_alias_target_in_module(current_module, name),
            [_, .., name] => {
                let use_decl =
                    imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
                self.schema_alias_target_in_module(Some(&use_decl.name), name)
            }
            _ => None,
        }
    }

    fn schema_alias_target_in_module(
        &self,
        module_name: Option<&str>,
        name: &str,
    ) -> Option<SchemaAliasTarget> {
        let alias = self
            .aliases
            .iter()
            .find(|alias| alias.name == name && alias.module_name.as_deref() == module_name)?;
        Some(SchemaAliasTarget {
            target: alias.target.clone(),
            module_name: alias.module_name.clone(),
        })
    }

    fn schema_target_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
        allow_private_local_schema: bool,
        companion_access_targets: &BTreeMap<String, String>,
        visited_aliases: &mut Vec<(Option<String>, String)>,
    ) -> Option<ResolvedSchemaSymbol> {
        match segments {
            [name] => self.schema_target_in_module(
                current_module,
                name,
                allow_private_local_schema,
                uses,
                companion_access_targets,
                visited_aliases,
            ),
            [_, .., name] => {
                let use_decl =
                    imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
                self.schema_target_in_module(
                    Some(&use_decl.name),
                    name,
                    companion_private_schema_access_allowed(
                        use_decl,
                        current_module,
                        companion_access_targets,
                    ),
                    uses,
                    companion_access_targets,
                    visited_aliases,
                )
            }
            _ => None,
        }
    }

    fn schema_target_in_module(
        &self,
        module_name: Option<&str>,
        name: &str,
        allow_private_schema: bool,
        uses: &[UseDecl],
        companion_access_targets: &BTreeMap<String, String>,
        visited_aliases: &mut Vec<(Option<String>, String)>,
    ) -> Option<ResolvedSchemaSymbol> {
        if let Some(schema) = self
            .schemas
            .iter()
            .find(|schema| schema.name == name && schema.module_name.as_deref() == module_name)
        {
            return (allow_private_schema || schema.visibility == Visibility::Public).then(|| {
                ResolvedSchemaSymbol {
                    name: schema.name.clone(),
                    module_name: schema.module_name.clone(),
                    span: schema.span.clone(),
                    unsupported_format_neutral_encode_field: schema
                        .unsupported_format_neutral_encode_field
                        .clone(),
                }
            });
        }
        let alias = self
            .aliases
            .iter()
            .find(|alias| alias.name == name && alias.module_name.as_deref() == module_name)?;
        let key = (alias.module_name.clone(), alias.name.clone());
        if visited_aliases.contains(&key) {
            return None;
        }
        visited_aliases.push(key);
        let result = self.schema_target_path(
            &alias.target,
            alias.module_name.as_deref(),
            uses,
            false,
            companion_access_targets,
            visited_aliases,
        );
        visited_aliases.pop();
        result
    }

    fn schema_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
        allow_private_local_schema: bool,
        companion_access_targets: &BTreeMap<String, String>,
        visited_aliases: &mut Vec<(Option<String>, String)>,
    ) -> SchemaPathLookup {
        match segments {
            [name] => self.schema_in_module(
                current_module,
                name,
                allow_private_local_schema,
                uses,
                companion_access_targets,
                visited_aliases,
            ),
            [_, .., name] => {
                let Some(use_decl) =
                    imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
                else {
                    return SchemaPathLookup::Missing;
                };
                self.schema_in_module(
                    Some(&use_decl.name),
                    name,
                    companion_private_schema_access_allowed(
                        use_decl,
                        current_module,
                        companion_access_targets,
                    ),
                    uses,
                    companion_access_targets,
                    visited_aliases,
                )
            }
            _ => SchemaPathLookup::Missing,
        }
    }

    fn schema_in_module(
        &self,
        module_name: Option<&str>,
        name: &str,
        allow_private_schema: bool,
        uses: &[UseDecl],
        companion_access_targets: &BTreeMap<String, String>,
        visited_aliases: &mut Vec<(Option<String>, String)>,
    ) -> SchemaPathLookup {
        if let Some(schema) = self
            .schemas
            .iter()
            .find(|schema| schema.name == name && schema.module_name.as_deref() == module_name)
        {
            return if allow_private_schema || schema.visibility == Visibility::Public {
                SchemaPathLookup::Visible
            } else {
                SchemaPathLookup::Private
            };
        }
        let Some(alias) = self
            .aliases
            .iter()
            .find(|alias| alias.name == name && alias.module_name.as_deref() == module_name)
        else {
            return SchemaPathLookup::Missing;
        };
        let key = (alias.module_name.clone(), alias.name.clone());
        if visited_aliases.contains(&key) {
            return SchemaPathLookup::Missing;
        }
        visited_aliases.push(key);
        let result = self.schema_path(
            &alias.target,
            alias.module_name.as_deref(),
            uses,
            false,
            companion_access_targets,
            visited_aliases,
        );
        visited_aliases.pop();
        result
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SchemaPathLookup {
    Visible,
    Private,
    Missing,
}

fn named_type_symbols(module: &SurfaceModule) -> Vec<NamedSymbol> {
    let mut symbols = module
        .types
        .iter()
        .filter_map(|ty| {
            Some(NamedSymbol {
                name: ty.name.clone()?,
                module_name: ty.module_name.clone(),
                visibility: ty.visibility,
            })
        })
        .collect::<Vec<_>>();
    symbols.extend(
        module
            .aliases
            .iter()
            .filter(|alias| alias.kind == PublicAliasKind::Type)
            .filter_map(|alias| {
                Some(NamedSymbol {
                    name: alias.name.clone()?,
                    module_name: alias.module_name.clone(),
                    visibility: Visibility::Public,
                })
            }),
    );
    symbols
}

fn named_codec_symbols(module: &SurfaceModule) -> Vec<NamedSymbol> {
    module
        .codecs
        .iter()
        .filter_map(|codec| {
            Some(NamedSymbol {
                name: codec.name.clone()?,
                module_name: codec.module_name.clone(),
                visibility: codec.visibility,
            })
        })
        .collect()
}

pub(crate) fn function_alias_signatures(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> Vec<FunctionSignature> {
    let companion_access_targets = BTreeMap::new();
    module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
        .filter_map(|alias| {
            let name = alias.name.clone()?;
            let target = function_signature_path(
                &alias.target,
                &module.uses,
                functions,
                alias.module_name.as_deref(),
                &companion_access_targets,
            )?;
            Some(FunctionSignature {
                name,
                target_name: target.target_name.clone(),
                module_name: alias.module_name.clone(),
                visibility: Visibility::Public,
                params: target.params.clone(),
                variadic: target.variadic.clone(),
                return_type: target.return_type.clone(),
                effects: target.effects.clone(),
                node_id: alias.node_id,
                span: alias.span.clone(),
            })
        })
        .collect()
}

fn function_signature_path<'a>(
    segments: &[String],
    uses: &[UseDecl],
    functions: &'a [FunctionSignature],
    current_module: Option<&str>,
    companion_access_targets: &BTreeMap<String, String>,
) -> Option<&'a FunctionSignature> {
    match segments {
        [name] => functions.iter().find(|function| function.name == *name),
        [_, .., name] => {
            let use_decl =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
            let module_name = use_decl.name.as_str();
            functions.iter().find(|function| {
                function.name == *name
                    && function.module_name.as_deref() == Some(module_name)
                    && imported_function_is_visible(
                        function,
                        use_decl,
                        current_module,
                        companion_access_targets,
                    )
            })
        }
        _ => None,
    }
}

fn collect_pattern_bindings(pattern: &Pattern, ty: &Type, bindings: &mut Vec<Binding>) {
    collect_let_pattern_bindings(pattern, ty, None, bindings);
}

fn collect_let_pattern_bindings(
    pattern: &Pattern,
    ty: &Type,
    private_function_value: Option<FunctionKey>,
    bindings: &mut Vec<Binding>,
) {
    match &pattern.kind {
        PatternKind::Binding(name) => bindings.push(match private_function_value {
            Some(target) => Binding::private_function_value(name.clone(), ty.clone(), target),
            None => Binding::new(name.clone(), ty.clone()),
        }),
        PatternKind::Record(fields) => {
            for field in fields {
                let field_ty = ty.record_field(&field.name).unwrap_or(&Type::Unknown);
                collect_let_pattern_bindings(&field.pattern, field_ty, None, bindings);
            }
        }
        PatternKind::Wildcard
        | PatternKind::StringLiteral(_)
        | PatternKind::IntLiteral(_)
        | PatternKind::FloatLiteral(_)
        | PatternKind::BoolLiteral(_)
        | PatternKind::Unit
        | PatternKind::Constructor { .. } => {}
    }
}

struct ExprEffectContext<'a> {
    uses: &'a [UseDecl],
    current_module: Option<&'a str>,
    bindings: &'a [Binding],
    functions: &'a [FunctionSignature],
    effects_by_function: &'a BTreeMap<(Option<String>, String), Vec<String>>,
    effects_by_module_path: &'a BTreeMap<(String, String), (Vec<String>, Visibility)>,
    companion_access_targets: &'a BTreeMap<String, String>,
    companion_effect_access_targets: &'a BTreeMap<String, CompanionAccessTarget>,
    user_effects: &'a [EffectSignature],
    handlers: &'a [HandlerSignature],
}

fn handler_for_path<'a>(
    segments: &[String],
    context: &ExprEffectContext<'a>,
) -> Option<&'a HandlerSignature> {
    match segments {
        [name] => context.handlers.iter().find(|handler| {
            handler.name == *name && handler.module_name.as_deref() == context.current_module
        }),
        [_, .., name] => {
            let use_decl = imported_use_for_path(
                context.uses,
                &segments[..segments.len() - 1],
                context.current_module,
            )?;
            context.handlers.iter().find(|handler| {
                handler.name == *name
                    && handler.module_name.as_deref() == Some(use_decl.name.as_str())
                    && imported_handler_is_visible(
                        handler,
                        use_decl,
                        context.current_module,
                        context.companion_effect_access_targets,
                    )
            })
        }
        _ => None,
    }
}

fn collect_expr_effect_dependencies(
    expr: &Expr,
    context: &ExprEffectContext<'_>,
    dependencies: &mut BTreeSet<EffectDependencyNode>,
) {
    ExprEffectDependencyCollector {
        context,
        dependencies,
    }
    .collect(expr);
}

struct ExprEffectDependencyCollector<'context, 'data, 'output> {
    context: &'context ExprEffectContext<'data>,
    dependencies: &'output mut BTreeSet<EffectDependencyNode>,
}

impl ExprEffectDependencyCollector<'_, '_, '_> {
    fn collect(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Call { callee, args } => self.collect_call(callee, args),
            ExprKind::Handle {
                body,
                handler,
                args,
                ..
            } => self.collect_handle(body, handler, args),
            ExprKind::SchemaDecode { input, base, .. } => self.collect_pair(input, base),
            ExprKind::Perform { args, .. } => self.collect_all(args),
            ExprKind::SchemaEncode { value, .. } => self.collect(value),
            ExprKind::FieldAccess { base, .. }
            | ExprKind::Try(base)
            | ExprKind::TypeApply { callee: base, .. }
            | ExprKind::Prefix { expr: base, .. } => self.collect(base),
            ExprKind::Record(fields) => {
                for field in fields {
                    self.collect(&field.expr);
                }
            }
            ExprKind::Dict(entries) => {
                for entry in entries {
                    self.collect_pair(&entry.key, &entry.value);
                }
            }
            ExprKind::List(items) => self.collect_all(items),
            ExprKind::Match { scrutinee, arms } => {
                self.collect(scrutinee);
                for arm in arms {
                    self.collect(&arm.expr);
                }
            }
            ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => {
                self.collect_pair(condition, then_branch);
                for branch in else_if_branches {
                    self.collect_pair(&branch.condition, &branch.expr);
                }
                self.collect(else_branch);
            }
            ExprKind::Binary { left, right, .. } => self.collect_pair(left, right),
            ExprKind::NamePath(segments) => self.collect_name_path(segments),
            ExprKind::Missing
            | ExprKind::Hole { .. }
            | ExprKind::StringLiteral(_)
            | ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::BoolLiteral(_)
            | ExprKind::Unit => {}
        }
    }

    fn collect_call(&mut self, callee: &Expr, args: &[Expr]) {
        if let Some(segments) = callee_name_path(callee) {
            self.collect_name_path(segments);
        } else {
            self.collect(callee);
        }
        self.collect_all(args);
    }

    fn collect_handle(&mut self, body: &Expr, handler: &[String], args: &[Expr]) {
        self.collect_all(args);
        if let Some(handler) = handler_for_path(handler, self.context)
            && handler.visibility != Visibility::Public
        {
            self.dependencies
                .insert(EffectDependencyNode::PrivateHandler(
                    handler.qualified_name.clone(),
                ));
        }
        self.collect(body);
    }

    fn collect_name_path(&mut self, segments: &[String]) {
        if let [name] = segments
            && let Some(target) = self
                .context
                .bindings
                .iter()
                .rev()
                .find(|binding| binding.name == *name)
                .and_then(|binding| binding.private_function_value.clone())
        {
            self.dependencies
                .insert(EffectDependencyNode::Function(target));
            return;
        }
        if let Some(signature) = function_signature_path(
            segments,
            self.context.uses,
            self.context.functions,
            self.context.current_module,
            self.context.companion_access_targets,
        ) {
            self.dependencies.insert(EffectDependencyNode::Function((
                signature.module_name.clone(),
                signature.name.clone(),
            )));
        }
    }

    fn collect_pair(&mut self, first: &Expr, second: &Expr) {
        self.collect(first);
        self.collect(second);
    }

    fn collect_all(&mut self, expressions: &[Expr]) {
        for expression in expressions {
            self.collect(expression);
        }
    }
}

fn collect_expr_effects(expr: &Expr, context: &ExprEffectContext<'_>, inferred: &mut Vec<String>) {
    ExprEffectCollector { context, inferred }.collect(expr);
}

struct ExprEffectCollector<'context, 'data, 'output> {
    context: &'context ExprEffectContext<'data>,
    inferred: &'output mut Vec<String>,
}

impl ExprEffectCollector<'_, '_, '_> {
    fn collect(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Call { callee, args } => self.collect_call(callee, args),
            ExprKind::SchemaDecode { input, base, .. } => self.collect_pair(input, base),
            ExprKind::Perform { effect, args, .. } => self.collect_perform(effect, args),
            ExprKind::Handle {
                body,
                handler,
                args,
                ..
            } => self.collect_handle(body, handler, args),
            ExprKind::SchemaEncode { value, .. } => self.collect(value),
            ExprKind::FieldAccess { base, .. }
            | ExprKind::Try(base)
            | ExprKind::TypeApply { callee: base, .. }
            | ExprKind::Prefix { expr: base, .. } => self.collect(base),
            ExprKind::Record(fields) => self.collect_record_fields(fields),
            ExprKind::Dict(entries) => self.collect_dict_entries(entries),
            ExprKind::List(items) => self.collect_all(items),
            ExprKind::Match { scrutinee, arms } => self.collect_match(scrutinee, arms),
            ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => self.collect_if(condition, then_branch, else_if_branches, else_branch),
            ExprKind::Binary { left, right, .. } => self.collect_pair(left, right),
            ExprKind::Missing
            | ExprKind::Hole { .. }
            | ExprKind::NamePath(_)
            | ExprKind::StringLiteral(_)
            | ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::BoolLiteral(_)
            | ExprKind::Unit => {}
        }
    }

    fn collect_call(&mut self, callee: &Expr, args: &[Expr]) {
        let Some(segments) = callee_name_path(callee) else {
            self.collect(callee);
            self.collect_all(args);
            return;
        };
        if is_stdio_call(segments) {
            push_unique_effect(self.inferred, "stdio");
        } else if let Some(effects) = concurrency_effects_for_call(segments, args, self.context) {
            self.push_all(&effects);
        } else if let Some(effects) = standard_library_effects(segments) {
            for effect in effects {
                push_unique_effect(self.inferred, effect);
            }
        } else if let [name] = segments.as_slice()
            && let Some(effects) = lexical_effects_for_bare_callee(
                name,
                self.context.bindings,
                self.context.effects_by_function,
            )
        {
            self.push_all(effects);
        } else if let Some(effects) = prelude_effects(segments) {
            for effect in effects {
                push_unique_effect(self.inferred, effect);
            }
        } else if let Some(signature) = function_signature_path(
            segments,
            self.context.uses,
            self.context.functions,
            self.context.current_module,
            self.context.companion_access_targets,
        ) {
            self.push_all(&instantiate_call_effect_rows(signature, args, self.context));
        } else {
            if let Some(effects) = effects_for_callee_path(
                segments,
                self.context.uses,
                self.context.current_module,
                self.context.bindings,
                self.context.effects_by_function,
                self.context.effects_by_module_path,
                self.context.companion_access_targets,
            ) {
                self.push_all(effects);
            }
        }
        self.collect_all(args);
    }

    fn collect_perform(&mut self, effect: &[String], args: &[Expr]) {
        if let Some(label) = canonical_user_effect_label(
            effect,
            self.context.uses,
            self.context.current_module,
            self.context.user_effects,
            self.context.companion_effect_access_targets,
        ) {
            push_unique_effect(self.inferred, &label);
        }
        self.collect_all(args);
    }

    fn collect_handle(&mut self, body: &Expr, handler: &[String], args: &[Expr]) {
        self.collect_all(args);
        let Some((handled_effect, handler_effects)) = handler_for_path(handler, self.context)
            .map(|handler| (handler.effect.clone(), handler.effects.clone()))
        else {
            self.collect(body);
            return;
        };
        let before_body = self.inferred.len();
        self.collect(body);
        let retained_body_effects = self
            .inferred
            .drain(before_body..)
            .filter(|effect| effect != &handled_effect)
            .collect::<Vec<_>>();
        self.inferred.extend(retained_body_effects);
        self.push_all(&handler_effects);
    }

    fn collect_pair(&mut self, first: &Expr, second: &Expr) {
        self.collect(first);
        self.collect(second);
    }

    fn collect_all(&mut self, expressions: &[Expr]) {
        for expression in expressions {
            self.collect(expression);
        }
    }

    fn collect_record_fields(&mut self, fields: &[RecordField]) {
        for field in fields {
            self.collect(&field.expr);
        }
    }

    fn collect_dict_entries(&mut self, entries: &[DictEntry]) {
        for entry in entries {
            self.collect_pair(&entry.key, &entry.value);
        }
    }

    fn collect_match(&mut self, scrutinee: &Expr, arms: &[MatchArm]) {
        self.collect(scrutinee);
        for arm in arms {
            self.collect(&arm.expr);
        }
    }

    fn collect_if(
        &mut self,
        condition: &Expr,
        then_branch: &Expr,
        else_if_branches: &[IfBranch],
        else_branch: &Expr,
    ) {
        self.collect_pair(condition, then_branch);
        for branch in else_if_branches {
            self.collect_pair(&branch.condition, &branch.expr);
        }
        self.collect(else_branch);
    }

    fn push_all(&mut self, effects: &[String]) {
        for effect in effects {
            push_unique_effect(self.inferred, effect);
        }
    }
}

fn instantiate_call_effect_rows(
    signature: &FunctionSignature,
    args: &[Expr],
    context: &ExprEffectContext<'_>,
) -> Vec<String> {
    let mut row_substitutions = Vec::<(String, Vec<String>)>::new();
    for (param, arg) in signature.params.iter().zip(args) {
        let Some(actual) = function_type_for_expr(arg, context) else {
            continue;
        };
        collect_effect_row_substitution_from_types(param, &actual, &mut row_substitutions);
    }
    instantiate_effect_row_entries(&signature.effects, &row_substitutions)
}

fn function_type_for_expr(expr: &Expr, context: &ExprEffectContext<'_>) -> Option<Type> {
    let segments = callee_name_path(expr)?;
    match segments.as_slice() {
        [name] => context
            .bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)
            .map(|binding| binding.ty.clone())
            .or_else(|| {
                function_signature_path(
                    segments,
                    context.uses,
                    context.functions,
                    context.current_module,
                    context.companion_access_targets,
                )
                .map(FunctionSignature::ty)
            }),
        _ => {
            let public_or_same_module_access = BTreeMap::new();
            function_signature_path(
                segments,
                context.uses,
                context.functions,
                context.current_module,
                &public_or_same_module_access,
            )
            .map(FunctionSignature::ty)
        }
    }
}

fn collect_effect_row_substitution_from_types(
    expected: &Type,
    actual: &Type,
    row_substitutions: &mut Vec<(String, Vec<String>)>,
) {
    let (
        Type::Function {
            params: expected_params,
            variadic: expected_variadic,
            return_type: expected_return,
            effects: expected_effects,
        },
        Type::Function {
            params: actual_params,
            variadic: actual_variadic,
            return_type: actual_return,
            effects: actual_effects,
        },
    ) = (expected, actual)
    else {
        return;
    };

    for effect in expected_effects {
        let Some(row) = effect.strip_prefix("...") else {
            continue;
        };
        let concrete = actual_effects
            .iter()
            .filter(|actual_effect| {
                !expected_effects
                    .iter()
                    .any(|expected_effect| expected_effect == *actual_effect)
            })
            .cloned()
            .collect::<Vec<_>>();
        merge_effect_row_substitution(row_substitutions, row, concrete);
    }

    for (expected_param, actual_param) in expected_params.iter().zip(actual_params) {
        collect_effect_row_substitution_from_types(expected_param, actual_param, row_substitutions);
    }
    if let (Some(expected), Some(actual)) =
        (expected_variadic.as_deref(), actual_variadic.as_deref())
    {
        collect_effect_row_substitution_from_types(expected, actual, row_substitutions);
    }
    collect_effect_row_substitution_from_types(expected_return, actual_return, row_substitutions);
}

fn merge_effect_row_substitution(
    row_substitutions: &mut Vec<(String, Vec<String>)>,
    row: &str,
    effects: Vec<String>,
) {
    if let Some((_, existing)) = row_substitutions
        .iter_mut()
        .find(|(existing_row, _)| existing_row == row)
    {
        for effect in effects {
            push_unique_effect(existing, &effect);
        }
        return;
    }
    let mut unique = Vec::new();
    for effect in effects {
        push_unique_effect(&mut unique, &effect);
    }
    row_substitutions.push((row.to_string(), unique));
}

fn instantiate_effect_row_entries(
    effects: &[String],
    row_substitutions: &[(String, Vec<String>)],
) -> Vec<String> {
    let mut instantiated = Vec::new();
    for effect in effects {
        if let Some(row) = effect.strip_prefix("...") {
            if let Some((_, substitution)) = row_substitutions
                .iter()
                .find(|(candidate, _)| candidate == row)
            {
                for substituted in substitution {
                    push_unique_effect(&mut instantiated, substituted);
                }
            } else {
                push_unique_effect(&mut instantiated, effect);
            }
        } else {
            push_unique_effect(&mut instantiated, effect);
        }
    }
    instantiated
}

fn callee_name_path(callee: &Expr) -> Option<&Vec<String>> {
    match &callee.kind {
        ExprKind::NamePath(segments) => Some(segments),
        ExprKind::TypeApply { callee, .. } => callee_name_path(callee),
        _ => None,
    }
}

fn concurrency_effects_for_call(
    segments: &[String],
    args: &[Expr],
    context: &ExprEffectContext<'_>,
) -> Option<Vec<String>> {
    let mut effects = concurrency_effects(segments)?
        .iter()
        .map(|effect| (*effect).to_string())
        .collect::<Vec<_>>();
    if matches!(segments, [module, name] if module == "task" && matches!(name.as_str(), "spawn" | "spawn_with"))
        && let Some(job_effects) = args
            .first()
            .and_then(callee_name_path)
            .and_then(|segments| {
                effects_for_callee_path(
                    segments,
                    context.uses,
                    context.current_module,
                    context.bindings,
                    context.effects_by_function,
                    context.effects_by_module_path,
                    context.companion_access_targets,
                )
            })
    {
        for effect in job_effects {
            push_unique_effect(&mut effects, effect);
        }
    }
    Some(effects)
}

fn effects_for_callee_path<'a>(
    segments: &[String],
    uses: &[UseDecl],
    current_module: Option<&str>,
    bindings: &'a [Binding],
    effects_by_function: &'a BTreeMap<(Option<String>, String), Vec<String>>,
    effects_by_module_path: &'a BTreeMap<(String, String), (Vec<String>, Visibility)>,
    companion_access_targets: &'a BTreeMap<String, String>,
) -> Option<&'a [String]> {
    match segments {
        [name] => effects_for_bare_callee(name, current_module, bindings, effects_by_function),
        [_, .., name] => {
            let use_decl =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
            effects_by_module_path
                .get(&(use_decl.name.clone(), name.clone()))
                .filter(|(_, visibility)| {
                    imported_effects_are_visible(
                        use_decl,
                        current_module,
                        use_decl.name.as_str(),
                        *visibility,
                        companion_access_targets,
                    )
                })
                .map(|(effects, _)| effects.as_slice())
        }
        _ => None,
    }
}

fn lexical_effects_for_bare_callee<'a>(
    name: &str,
    bindings: &'a [Binding],
    effects_by_function: &'a BTreeMap<(Option<String>, String), Vec<String>>,
) -> Option<&'a [String]> {
    let binding = bindings.iter().rev().find(|binding| binding.name == name)?;
    if let Some(target) = &binding.private_function_value {
        return effects_by_function.get(target).map(Vec::as_slice);
    }
    Some(binding.ty.function_effects().unwrap_or(&[]))
}

pub(crate) fn imported_use_for_path<'a>(
    uses: &'a [UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let module_path = segments.join("::");
    uses.iter().find(|use_decl| {
        use_decl.module_name.as_deref() == current_module
            && (use_decl.name == module_path || use_decl.alias == module_path)
    })
}

fn imported_function_is_visible(
    function: &FunctionSignature,
    use_decl: &UseDecl,
    current_module: Option<&str>,
    companion_access_targets: &BTreeMap<String, String>,
) -> bool {
    if function.visibility == Visibility::Public {
        return true;
    }
    if use_decl.package.is_none()
        && current_module.is_some_and(|module| module.starts_with("std::"))
        && function
            .module_name
            .as_deref()
            .is_some_and(|module| module.starts_with("std::"))
    {
        return true;
    }
    use_decl.package.is_none()
        && current_module.is_some_and(|current_module| {
            function.module_name.as_ref().is_some_and(|target_module| {
                companion_access_targets
                    .get(current_module)
                    .is_some_and(|allowed| allowed == target_module)
            })
        })
}

fn imported_effects_are_visible(
    use_decl: &UseDecl,
    current_module: Option<&str>,
    target_module: &str,
    visibility: Visibility,
    companion_access_targets: &BTreeMap<String, String>,
) -> bool {
    visibility == Visibility::Public
        || (use_decl.package.is_none()
            && current_module.is_some_and(|current_module| {
                (current_module.starts_with("std::") && target_module.starts_with("std::"))
                    || companion_access_targets
                        .get(current_module)
                        .is_some_and(|allowed| allowed == target_module)
            }))
}

fn imported_handler_is_visible(
    handler: &HandlerSignature,
    use_decl: &UseDecl,
    current_module: Option<&str>,
    companion_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> bool {
    handler.visibility == Visibility::Public
        || (use_decl.package.is_none()
            && current_module.is_some_and(|current_module| {
                handler.module_name.as_deref().is_some_and(|target_module| {
                    (current_module.starts_with("std::") && target_module.starts_with("std::"))
                        || companion_access_targets
                            .get(current_module)
                            .is_some_and(|access| access.target_module == target_module)
                })
            }))
}

fn companion_private_schema_access_allowed(
    use_decl: &UseDecl,
    current_module: Option<&str>,
    companion_access_targets: &BTreeMap<String, String>,
) -> bool {
    use_decl.package.is_none()
        && current_module.is_some_and(|current_module| {
            companion_access_targets
                .get(current_module)
                .is_some_and(|allowed| allowed == use_decl.name.as_str())
        })
}

fn companion_access_targets(module: &SurfaceModule) -> BTreeMap<String, String> {
    module
        .functions
        .iter()
        .filter_map(|function| {
            companion_access_target(function.span.file.as_str(), function.module_name.as_deref())
        })
        .chain(module.schemas.iter().filter_map(|schema| {
            companion_access_target(schema.span.file.as_str(), schema.module_name.as_deref())
        }))
        .collect()
}

fn companion_access_target(path: &str, module_name: Option<&str>) -> Option<(String, String)> {
    let companion = classify_companion_source(path)?;
    let companion_module = module_name?.to_string();
    let target_module = companion
        .target_path
        .strip_suffix(".veln")?
        .replace('/', "::");
    Some((companion_module, target_module))
}

fn companion_access_target_infos(
    module: &SurfaceModule,
) -> BTreeMap<String, CompanionAccessTarget> {
    module
        .functions
        .iter()
        .filter_map(|function| {
            companion_access_target_info(
                function.span.file.as_str(),
                function.module_name.as_deref(),
            )
        })
        .chain(module.handlers.iter().filter_map(|handler| {
            companion_access_target_info(handler.span.file.as_str(), handler.module_name.as_deref())
        }))
        .chain(module.effects.iter().filter_map(|effect| {
            companion_access_target_info(effect.span.file.as_str(), effect.module_name.as_deref())
        }))
        .collect()
}

fn companion_access_target_info(
    path: &str,
    module_name: Option<&str>,
) -> Option<(String, CompanionAccessTarget)> {
    let companion = classify_companion_source(path)?;
    let companion_module = module_name?.to_string();
    let target_module = companion
        .target_path
        .strip_suffix(".veln")?
        .replace('/', "::");
    Some((
        companion_module,
        CompanionAccessTarget {
            companion_path: companion.companion_path,
            target_module,
        },
    ))
}

fn companion_function_access_targets(module: &SurfaceModule) -> BTreeMap<String, String> {
    module
        .functions
        .iter()
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

fn companion_access_targets_for_signatures(
    functions: &[FunctionSignature],
) -> BTreeMap<String, String> {
    functions
        .iter()
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

fn effects_for_bare_callee<'a>(
    name: &str,
    current_module: Option<&str>,
    bindings: &'a [Binding],
    effects_by_function: &'a BTreeMap<(Option<String>, String), Vec<String>>,
) -> Option<&'a [String]> {
    if let Some(effects) = lexical_effects_for_bare_callee(name, bindings, effects_by_function) {
        return Some(effects);
    }
    if let Some(current_module) = current_module {
        return effects_by_function
            .get(&(Some(current_module.to_string()), name.to_string()))
            .map(Vec::as_slice);
    }
    effects_by_function
        .get(&(None, name.to_string()))
        .map(Vec::as_slice)
}

fn push_unique_effect(effects: &mut Vec<String>, effect: &str) {
    if !effects.iter().any(|existing| existing == effect) {
        effects.push(effect.to_string());
    }
}

#[cfg(test)]
#[path = "types/tests.rs"]
mod tests;
