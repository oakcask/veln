use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StandardSemanticIdentity {
    bundle_hash: u64,
    semantic_model: &'static str,
}

#[derive(Clone)]
pub struct ReusableStandardEnvironment {
    pub(super) identity: StandardSemanticIdentity,
    module_names: BTreeSet<String>,
    declaration_counts: BTreeMap<StandardDeclarationKey, usize>,
    environment: Arc<TypeEnvironment>,
}

const STANDARD_SEMANTIC_MODEL: &str = "standard-semantic-signatures-v1";

impl ReusableStandardEnvironment {
    pub(super) fn environment_for_modules(
        &self,
        module_names: &BTreeSet<String>,
    ) -> Arc<TypeEnvironment> {
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

    pub(crate) fn record_application_prepare() {
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

pub(super) fn application_module_is_empty(module: &SurfaceModule) -> bool {
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

pub(super) fn selected_standard_facts<T: Clone>(
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

pub(super) fn selected_standard_access_targets(
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

pub(super) fn module_without_reusable_standard_declarations(
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

pub(super) fn reusable_standard_module_names_for(module: &SurfaceModule) -> BTreeSet<String> {
    module_standard_names(&module_without_application_declarations(module))
}

fn module_without_application_declarations(module: &SurfaceModule) -> SurfaceModule {
    filter_module_declarations(module, is_embedded_standard_declaration)
}

pub(super) fn module_standard_names(module: &SurfaceModule) -> BTreeSet<String> {
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

pub(super) fn is_standard_module_name(module_name: Option<&str>) -> bool {
    module_name.is_some_and(|module_name| module_name.starts_with("std::"))
}

pub(super) fn valid_value_binding_name(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
}
