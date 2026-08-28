pub(crate) mod environment;
pub(crate) mod private_inference;
pub(crate) mod schema_types;
pub(crate) mod signatures;
mod symbols;

pub(crate) use environment::*;
pub(crate) use private_inference::*;
use private_inference::{
    collect_pattern_bindings, function_signature_params, function_signature_path,
};
pub(crate) use schema_types::*;
pub(crate) use signatures::*;
use symbols::*;

use crate::schema::*;

#[cfg(test)]
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

use veln_ast::{
    BodyLineKind, CodecDecl, CodecDirection, CodecImplementationKind, DictEntry, EffectDecl, Expr,
    ExprKind, Function, FunctionKind, HandlerDecl, IfBranch, MatchArm, PublicAlias,
    PublicAliasKind, RecordField, SchemaDecl, SchemaField, SurfaceModule, TypeDecl, UseDecl,
    Visibility, lower_surface_ast_with_module_identity,
};
use veln_project::classify_companion_source;
use veln_source::{SourceFile, SourceSpan, TextRange};

use crate::adt::{self, AdtRegistry};
use crate::effects::{
    concurrency_effects, is_stdio_call, prelude_effects, standard_library_effects,
};
use crate::name_recovery::normal_use_decls;
use crate::semantic_model::{Binding, FunctionKey, Type};
use crate::type_syntax::{parse_type_annotation, parse_type_or_unknown};

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
        invalid_names: Vec::new(),
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
    merged.invalid_names.extend(module.invalid_names);
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
        invalid_names: module.invalid_names.clone(),
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

fn valid_value_binding_name(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
}

pub(crate) fn ordinary_function_signatures(
    module: &SurfaceModule,
    effects: &[EffectSignature],
    adts: &AdtRegistry,
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Vec<FunctionSignature> {
    let uses = normal_use_decls(module);
    let quarantined_uses = module
        .uses
        .iter()
        .filter(|use_decl| {
            crate::name_recovery::use_decl_has_invalid_module_segment(module, use_decl)
        })
        .cloned()
        .collect::<Vec<_>>();
    module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(|function| {
            let name = function.name.clone()?;
            if !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase) {
                return None;
            }
            let (params, variadic) = function_signature_params(function);
            let params = params
                .into_iter()
                .map(|ty| {
                    canonicalize_type_effects(
                        ty,
                        &uses,
                        &quarantined_uses,
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
                    &uses,
                    &quarantined_uses,
                    function.module_name.as_deref(),
                    effects,
                    adts,
                    companion_effect_access_targets,
                )
            });
            let return_type = canonicalize_type_effects(
                parse_type_or_unknown(function.return_type.as_deref()),
                &uses,
                &quarantined_uses,
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
                    &uses,
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
    quarantined_uses: &[UseDecl],
    current_module: Option<&str>,
    effects: &[EffectSignature],
    adts: &AdtRegistry,
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Type {
    match ty {
        Type::Named { name, args } => {
            let Some(canonical_name) = adts
                .descriptor_for_type_path(&name, args.len(), current_module, uses)
                .map(|descriptor| descriptor.type_name.clone())
                .or_else(|| {
                    canonical_type_name_without_descriptor(
                        &name,
                        current_module,
                        uses,
                        quarantined_uses,
                        args.len(),
                        adts,
                    )
                })
            else {
                return Type::Unknown;
            };
            Type::Named {
                name: canonical_name,
                args: args
                    .into_iter()
                    .map(|arg| {
                        canonicalize_type_effects(
                            arg,
                            uses,
                            quarantined_uses,
                            current_module,
                            effects,
                            adts,
                            companion_effect_access_targets,
                        )
                    })
                    .collect(),
            }
        }
        Type::Record(fields) => Type::Record(
            fields
                .into_iter()
                .map(|(name, ty)| {
                    (
                        name,
                        canonicalize_type_effects(
                            ty,
                            uses,
                            quarantined_uses,
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
                        quarantined_uses,
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
                        quarantined_uses,
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
                quarantined_uses,
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

fn canonical_type_name_without_descriptor(
    name: &str,
    current_module: Option<&str>,
    uses: &[UseDecl],
    quarantined_uses: &[UseDecl],
    args_len: usize,
    adts: &AdtRegistry,
) -> Option<String> {
    if !name.contains("::") {
        return Some(name.to_string());
    }
    let segments = name.split("::").map(str::to_string).collect::<Vec<_>>();
    match segments.as_slice() {
        [_, .., _] => {
            if imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
                .is_some()
            {
                return Some(name.to_string());
            }
            let use_decl = imported_use_for_path(
                quarantined_uses,
                &segments[..segments.len() - 1],
                current_module,
            )?;
            adts.descriptor_for_type_path(name, args_len, current_module, quarantined_uses)
                .filter(|descriptor| {
                    descriptor.module_name.as_deref() == Some(use_decl.name.as_str())
                        && descriptor.visibility == Visibility::Public
                })
                .map_or_else(|| Some(name.to_string()), |_| None)
        }
        _ => Some(name.to_string()),
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
    let uses = normal_use_decls(module);
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
                &uses,
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
                    &uses,
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
    uses: &'a [UseDecl],
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
    uses: &'a [UseDecl],
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
    uses: Vec<UseDecl>,
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
            uses: normal_use_decls(module),
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
                uses: &self.uses,
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
            uses: &self.uses,
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
    let uses = normal_use_decls(module);
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
        uses: &uses,
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
        .filter(|(_, param)| valid_value_binding_name(&param.name))
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
                .filter(|param| valid_value_binding_name(&param.name))
                .map(|param| Binding::new(param.name.clone(), Type::Unknown)),
        );
        let expr_context = ExprEffectContext {
            uses: context.uses,
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
            .filter(|(_, param)| valid_value_binding_name(&param.name))
            .map(|(index, param)| {
                Binding::new(
                    param.name.clone(),
                    handler.params.get(index).cloned().unwrap_or(Type::Unknown),
                )
            })
            .collect::<Vec<_>>();
        bindings.extend(
            clause
                .params
                .iter()
                .enumerate()
                .filter_map(|(index, param)| {
                    if valid_value_binding_name(&param.name) {
                        Some(Binding::new(
                            param.name.clone(),
                            operation
                                .params
                                .get(index)
                                .cloned()
                                .unwrap_or(Type::Unknown),
                        ))
                    } else {
                        None
                    }
                }),
        );
        let expr_context = ExprEffectContext {
            uses: context.uses,
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
        .filter(|param| valid_value_binding_name(&param.name))
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
                    uses: context.uses,
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
                    uses: context.uses,
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
        .filter(|param| valid_value_binding_name(&param.name))
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
                    uses: context.uses,
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
                    uses: context.uses,
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
