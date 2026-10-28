use super::*;
use veln_ast::NameOccurrence;

use crate::adt::registry::AdtRegistry;

type FunctionPublicationKey = (
    Option<String>,
    Option<String>,
    String,
    SourcePath,
    usize,
    usize,
);

type BareModuleValueKey = (Option<String>, Option<String>, String);
type QualifiedModuleValueKey = (veln_ast::ResolvedImportOwner, String);

#[derive(Clone, Debug)]
enum QualifiedModuleValueImportSelection {
    Unique(Box<UseDecl>),
    Ambiguous { visible_members: HashSet<String> },
}

#[derive(Clone, Debug)]
enum BareModuleValueSelection {
    Unique(ModuleValueIdentity),
    Ambiguous,
}

#[derive(Clone, Debug)]
struct BareModuleValueExposure {
    selection: BareModuleValueSelection,
    function_collision: bool,
}

#[cfg(test)]
thread_local! {
    static BARE_MODULE_VALUE_LOOKUP_WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static QUALIFIED_MODULE_VALUE_IMPORT_WORK: std::cell::Cell<QualifiedModuleValueImportWork> = const { std::cell::Cell::new(QualifiedModuleValueImportWork::ZERO) };
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct QualifiedModuleValueImportWork {
    pub(crate) index_build: usize,
    pub(crate) target_classification: usize,
    pub(crate) member_materialization: usize,
    pub(crate) same_owner_blocking: usize,
    pub(crate) selection: usize,
    pub(crate) visibility: usize,
}

#[cfg(test)]
impl QualifiedModuleValueImportWork {
    const ZERO: Self = Self {
        index_build: 0,
        target_classification: 0,
        member_materialization: 0,
        same_owner_blocking: 0,
        selection: 0,
        visibility: 0,
    };
}

#[cfg(test)]
pub(crate) fn reset_bare_module_value_lookup_work() {
    BARE_MODULE_VALUE_LOOKUP_WORK.with(|work| work.set(0));
}

#[cfg(test)]
pub(crate) fn take_bare_module_value_lookup_work() -> usize {
    BARE_MODULE_VALUE_LOOKUP_WORK.with(|work| work.replace(0))
}

#[cfg(test)]
pub(crate) fn reset_qualified_module_value_import_work() {
    QUALIFIED_MODULE_VALUE_IMPORT_WORK.with(|work| work.set(QualifiedModuleValueImportWork::ZERO));
}

#[cfg(test)]
pub(crate) fn take_qualified_module_value_import_work() -> QualifiedModuleValueImportWork {
    QUALIFIED_MODULE_VALUE_IMPORT_WORK
        .with(|work| work.replace(QualifiedModuleValueImportWork::ZERO))
}

fn record_qualified_module_value_import_build_work() {
    #[cfg(test)]
    QUALIFIED_MODULE_VALUE_IMPORT_WORK.with(|work| {
        let mut value = work.get();
        value.index_build += 1;
        work.set(value);
    });
}

fn record_qualified_module_value_target_classification_work() {
    #[cfg(test)]
    QUALIFIED_MODULE_VALUE_IMPORT_WORK.with(|work| {
        let mut value = work.get();
        value.target_classification += 1;
        work.set(value);
    });
}

fn record_qualified_module_value_member_materialization_work() {
    #[cfg(test)]
    QUALIFIED_MODULE_VALUE_IMPORT_WORK.with(|work| {
        let mut value = work.get();
        value.member_materialization += 1;
        work.set(value);
    });
}

fn record_qualified_module_value_same_owner_work() {
    #[cfg(test)]
    QUALIFIED_MODULE_VALUE_IMPORT_WORK.with(|work| {
        let mut value = work.get();
        value.same_owner_blocking += 1;
        work.set(value);
    });
}

fn record_qualified_module_value_selection_work() {
    #[cfg(test)]
    QUALIFIED_MODULE_VALUE_IMPORT_WORK.with(|work| {
        let mut value = work.get();
        value.selection += 1;
        work.set(value);
    });
}

fn record_qualified_module_value_visibility_work() {
    #[cfg(test)]
    QUALIFIED_MODULE_VALUE_IMPORT_WORK.with(|work| {
        let mut value = work.get();
        value.visibility += 1;
        work.set(value);
    });
}

fn record_bare_module_value_lookup_work() {
    #[cfg(test)]
    BARE_MODULE_VALUE_LOOKUP_WORK.with(|work| work.set(work.get() + 1));
}

fn function_publication_key(function: &FunctionSignature) -> FunctionPublicationKey {
    (
        function.package_name.clone(),
        function.module_name.clone(),
        function.name.clone(),
        function.span.file.clone(),
        function.span.start.offset,
        function.span.end.offset,
    )
}

fn source_function_publication_key(function: &Function) -> FunctionPublicationKey {
    (
        function.package_name.clone(),
        function.module_name.clone(),
        function.name.clone().unwrap_or_default(),
        function.span.file.clone(),
        function.span.start.offset,
        function.span.end.offset,
    )
}

pub(crate) struct TypeParameterIdentityContext<'a> {
    positions: HashMap<&'a str, usize>,
    lookups: Option<std::cell::Cell<usize>>,
}

impl<'a> TypeParameterIdentityContext<'a> {
    pub(crate) fn new(type_parameters: &'a [String]) -> Self {
        Self::with_lookup_recording(type_parameters, false)
    }

    pub(crate) fn recording(type_parameters: &'a [String]) -> Self {
        Self::with_lookup_recording(type_parameters, true)
    }

    fn with_lookup_recording(type_parameters: &'a [String], record_lookups: bool) -> Self {
        Self {
            positions: type_parameters
                .iter()
                .enumerate()
                .map(|(index, parameter)| (parameter.as_str(), index))
                .collect(),
            lookups: record_lookups.then(|| std::cell::Cell::new(0)),
        }
    }

    pub(crate) fn ordinal(&self, name: &str) -> Option<usize> {
        if let Some(lookups) = &self.lookups {
            lookups.set(lookups.get() + 1);
        }
        self.positions.get(name).copied()
    }

    pub(crate) fn identity(&self, name: &str) -> Option<String> {
        self.ordinal(name)
            .map(|ordinal| format!("type-parameter:{ordinal}"))
    }

    pub(crate) fn is_identity(identity: &str) -> bool {
        identity.starts_with("type-parameter:")
    }

    pub(crate) fn name_count(&self) -> usize {
        self.positions.len()
    }

    pub(crate) fn lookup_count(&self) -> usize {
        self.lookups.as_ref().map_or(0, std::cell::Cell::get)
    }
}

mod effect_handlers;
mod facts;
mod quarantined_imports;
mod refinement_case_recovery;
mod refinements;
mod standard_subset;

#[derive(Clone)]
pub(crate) struct TypeEnvironment {
    resolution_owner: Option<veln_ast::ResolvedImportOwner>,
    module_values: Vec<ModuleValueSignature>,
    module_values_by_identity: HashMap<ModuleValueIdentity, ModuleValueIndex>,
    bare_module_values: HashMap<BareModuleValueKey, BareModuleValueExposure>,
    qualified_module_value_imports:
        HashMap<QualifiedModuleValueKey, QualifiedModuleValueImportSelection>,
    exact_module_value_imports: HashSet<QualifiedModuleValueKey>,
    blocked_exact_module_value_imports: HashSet<QualifiedModuleValueKey>,
    public_module_value_identities: HashSet<ModuleValueIdentity>,
    functions: Vec<FunctionSignature>,
    functions_by_name: HashMap<String, Vec<usize>>,
    unpublished_functions: HashSet<FunctionPublicationKey>,
    function_recovery_signatures: Vec<FunctionSignature>,
    function_recoveries: BTreeMap<FunctionRecoveryKey, usize>,
    constructor_recoveries: BTreeMap<ConstructorRecoveryKey, usize>,
    import_constructor_recoveries: BTreeMap<ImportConstructorRecoveryKey, usize>,
    effects: Vec<EffectSignature>,
    handlers: Vec<HandlerSignature>,
    schema_symbols: SchemaSymbolTable,
    type_symbols: Vec<NamedSymbol>,
    pub(crate) uses: Vec<UseDecl>,
    quarantined_uses: Vec<UseDecl>,
    invalid_names: Vec<InvalidName>,
    pub(crate) adts: AdtRegistry,
    companion_function_access_targets: BTreeMap<String, String>,
    companion_schema_access_targets: BTreeMap<String, String>,
    companion_effect_access_targets: BTreeMap<String, CompanionAccessTarget>,
}

pub(crate) struct VariantRefinementBaseFailure {
    pub(crate) reason: &'static str,
    pub(crate) resolved_identity: String,
    pub(crate) declaration_span: Option<SourceSpan>,
    pub(crate) related_kind: &'static str,
    pub(crate) provider: Option<&'static str>,
    pub(crate) related_message: String,
}

impl TypeEnvironment {
    pub(crate) fn with_resolution_owner(
        &self,
        module: Option<&str>,
        package: Option<&str>,
    ) -> Self {
        let mut environment = self.clone();
        environment.resolution_owner = Some(veln_ast::ResolvedImportOwner {
            package: package.map(str::to_string),
            module: module.map(str::to_string),
        });
        environment
    }

    fn resolution_package<'a>(&'a self, current_module: Option<&str>) -> Option<&'a str> {
        self.resolution_owner
            .as_ref()
            .filter(|owner| owner.module.as_deref() == current_module)
            .and_then(|owner| owner.package.as_deref())
    }

    pub(crate) fn quarantine_module_values(&mut self, invalid: &BTreeSet<ModuleValueIdentity>) {
        for value in &mut self.module_values {
            value.available = value.declaration.name.as_ref().is_some_and(|name| {
                !invalid.contains(&(
                    value.declaration.package_name.clone(),
                    value.declaration.module_name.clone(),
                    name.clone(),
                ))
            });
        }
        self.rebuild_module_value_exposures();
    }

    fn rebuild_module_value_exposures(&mut self) {
        self.public_module_value_identities =
            facts::public_module_value_identities(&self.module_values);
        self.qualified_module_value_imports = facts::qualified_module_value_import_index(
            &self.uses,
            &self.module_values,
            &self.companion_function_access_targets,
        );
        self.bare_module_values =
            facts::bare_module_value_index(&self.module_values, &self.functions, &self.uses);
    }

    pub(crate) fn module_value_path_in_owner(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> ModuleValueLookup<'_> {
        let Some((name, _)) = segments.split_last() else {
            return ModuleValueLookup::Missing;
        };
        if let Some(value) =
            self.same_owner_module_value(segments, name, current_module, current_package)
        {
            return value;
        }

        if segments.len() == 1 {
            return self.bare_module_value_in_owner(name, current_module, current_package);
        }

        self.qualified_module_value_in_owner(segments, name, current_module, current_package)
    }

    fn same_owner_module_value<'a>(
        &'a self,
        segments: &[String],
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<ModuleValueLookup<'a>> {
        self.same_module_value_path_matches_in_owner(segments, current_module, current_package)
            .then(|| {
                self.module_values_by_identity.get(&(
                    current_package.map(str::to_owned),
                    current_module.map(str::to_owned),
                    name.to_owned(),
                ))
            })
            .flatten()
            .map(|index| self.module_value_at_index(*index))
    }

    fn bare_module_value_in_owner(
        &self,
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> ModuleValueLookup<'_> {
        record_bare_module_value_lookup_work();
        if self.functions_named(name).any(|function| {
            function.module_name.as_deref() == current_module
                && function.package_name.as_deref() == current_package
        }) {
            return ModuleValueLookup::Missing;
        }
        let key = (
            current_package.map(str::to_owned),
            current_module.map(str::to_owned),
            name.to_owned(),
        );
        let Some(exposure) = self.bare_module_values.get(&key) else {
            return ModuleValueLookup::Missing;
        };
        if matches!(exposure.selection, BareModuleValueSelection::Ambiguous)
            || exposure.function_collision
        {
            return ModuleValueLookup::Ambiguous;
        }
        let BareModuleValueSelection::Unique(identity) = &exposure.selection else {
            unreachable!();
        };
        self.module_values_by_identity
            .get(identity)
            .map_or(ModuleValueLookup::Missing, |index| {
                self.module_value_at_index(*index)
            })
    }

    fn qualified_module_value_in_owner(
        &self,
        segments: &[String],
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> ModuleValueLookup<'_> {
        let module_path = segments[..segments.len() - 1].join("::");
        let owner = veln_ast::ResolvedImportOwner {
            package: current_package.map(str::to_string),
            module: current_module.map(str::to_string),
        };
        let key = (owner, module_path);
        record_qualified_module_value_selection_work();
        let exact_path_is_blocked = self.blocked_exact_module_value_imports.contains(&key);
        record_qualified_module_value_selection_work();
        let selection = self.qualified_module_value_imports.get(&key);
        if exact_path_is_blocked && !self.exact_module_value_imports.contains(&key) {
            return ModuleValueLookup::Missing;
        }
        // A blocked exact path with selectable package-distinct exact targets remains
        // ambiguous. Without one, return above instead of using an implicit or local
        // fallback.
        let Some(selection) = selection else {
            return ModuleValueLookup::Missing;
        };
        let use_decl = match selection {
            QualifiedModuleValueImportSelection::Unique(use_decl) => use_decl,
            QualifiedModuleValueImportSelection::Ambiguous { visible_members } => {
                record_qualified_module_value_visibility_work();
                return if visible_members.contains(name) {
                    ModuleValueLookup::Ambiguous
                } else {
                    ModuleValueLookup::Missing
                };
            }
        };
        record_qualified_module_value_selection_work();
        let Some(index) = self.visible_module_value_index_for_use(use_decl, name, current_module)
        else {
            return ModuleValueLookup::Missing;
        };
        self.module_value_at_index(index)
    }

    fn visible_module_value_index_for_use(
        &self,
        use_decl: &UseDecl,
        name: &str,
        current_module: Option<&str>,
    ) -> Option<ModuleValueIndex> {
        let identity = (
            use_decl.resolved_target().package,
            Some(use_decl.name.clone()),
            name.to_owned(),
        );
        let index = *self.module_values_by_identity.get(&identity)?;
        let private_access =
            self.companion_private_module_value_access_allowed(use_decl, current_module);
        record_qualified_module_value_visibility_work();
        let visible = self.public_module_value_identities.contains(&identity) || private_access;
        visible.then_some(index)
    }

    fn module_value_at_index(&self, index: ModuleValueIndex) -> ModuleValueLookup<'_> {
        match index {
            ModuleValueIndex::Unique(index) => {
                let value = &self.module_values[index];
                if value.available {
                    ModuleValueLookup::Available(value)
                } else {
                    ModuleValueLookup::Unavailable
                }
            }
            ModuleValueIndex::Collision => ModuleValueLookup::Unavailable,
        }
    }

    fn companion_private_module_value_access_allowed(
        &self,
        use_decl: &UseDecl,
        current_module: Option<&str>,
    ) -> bool {
        use_decl.package.is_none()
            && current_module.is_some_and(|current_module| {
                self.companion_function_access_targets
                    .get(current_module)
                    .is_some_and(|allowed_target| allowed_target == &use_decl.name)
            })
    }

    pub(crate) fn resolves_available_module_value_alias_target(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> bool {
        matches!(
            self.module_value_path_in_owner(segments, current_module, current_package),
            ModuleValueLookup::Available(_)
        )
    }

    pub(crate) fn same_module_value_path_matches_in_owner(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> bool {
        let Some((_, qualifier)) = segments.split_last() else {
            return false;
        };
        if qualifier.is_empty() {
            return true;
        }
        let Some(module) = current_module else {
            return false;
        };
        let qualifier = qualifier.join("::");
        record_qualified_module_value_same_owner_work();
        let key = (
            veln_ast::ResolvedImportOwner {
                package: current_package.map(str::to_string),
                module: current_module.map(str::to_string),
            },
            qualifier.clone(),
        );
        if self.blocked_exact_module_value_imports.contains(&key)
            || self.exact_module_value_imports.contains(&key)
        {
            return false;
        }
        qualifier == module
            || (!qualifier.contains("::") && module.rsplit("::").next() == Some(qualifier.as_str()))
    }

    pub(crate) fn invalid_cased_path_segment<'a>(
        &'a self,
        segments: &[String],
        segment_spans: &[SourceSpan],
        enclosing_function_span: &SourceSpan,
    ) -> Option<&'a InvalidName> {
        self.invalid_names.iter().find(|invalid| {
            invalid.occurrence == NameOccurrence::PathSegment
                && invalid
                    .enclosing_function_span
                    .as_ref()
                    .is_some_and(|span| {
                        span.file == enclosing_function_span.file
                            && span.start.offset == enclosing_function_span.start.offset
                            && span.end.offset == enclosing_function_span.end.offset
                    })
                && invalid
                    .segment_index
                    .is_some_and(|index| index < segments.len() && index < segment_spans.len())
                && invalid.segment_index.is_some_and(|index| {
                    invalid.name == segments[index]
                        && invalid.span.file == segment_spans[index].file
                        && invalid.span.start.offset == segment_spans[index].start.offset
                        && invalid.span.end.offset == segment_spans[index].end.offset
                })
        })
    }

    fn functions_named(&self, name: &str) -> impl Iterator<Item = &FunctionSignature> {
        self.functions_by_name
            .get(name)
            .into_iter()
            .flatten()
            .map(|index| &self.functions[*index])
    }

    pub(crate) fn from_module(module: &SurfaceModule) -> Self {
        Self::from_module_with_base(module, None)
    }

    // Path roles depend on declaration identity, visibility, and arity, not inferred
    // parameter/return types or effects. Do not use this environment for typechecking.
    pub(crate) fn for_path_classification(module: &SurfaceModule) -> Self {
        facts::from_module_for_path_classification(module)
    }

    fn for_path_classification_with_base(
        module: &SurfaceModule,
        base: Option<&TypeEnvironment>,
    ) -> Self {
        facts::from_module_for_path_classification_with_base(module, base)
    }

    #[cfg(test)]
    pub(crate) fn from_module_with_base_for_test(
        module: &SurfaceModule,
        base: &TypeEnvironment,
    ) -> Self {
        Self::from_module_with_base(module, Some(base))
    }

    pub(crate) fn from_module_with_standard(
        module: &SurfaceModule,
        standard: &ReusableStandardEnvironment,
    ) -> Self {
        if standard.identity != standard_semantic_identity() {
            return Self::from_module(module);
        }
        let application_module = module_without_reusable_standard_declarations(module, standard);
        let standard_module_names = reusable_standard_module_names_for(module);
        let standard_environment = standard.environment_for_modules(&standard_module_names);
        if application_module_is_empty(&application_module) {
            return standard_environment.as_ref().clone();
        }
        #[cfg(test)]
        standard_reuse_counters::record_application_prepare();
        Self::from_module_with_base(&application_module, Some(standard_environment.as_ref()))
    }

    pub(crate) fn from_application_module_with_standard(
        application_module: &SurfaceModule,
        selected_standard_module: &SurfaceModule,
        standard: &ReusableStandardEnvironment,
    ) -> Self {
        let standard_module_names = module_standard_names(selected_standard_module);
        Self::from_application_module_with_standard_module_names(
            application_module,
            &standard_module_names,
            standard,
        )
    }

    pub(crate) fn from_application_module_with_standard_module_names(
        application_module: &SurfaceModule,
        standard_module_names: &BTreeSet<String>,
        standard: &ReusableStandardEnvironment,
    ) -> Self {
        if standard.identity != standard_semantic_identity() {
            return Self::from_module(application_module);
        }
        let application_module =
            module_without_reusable_standard_declarations(application_module, standard);
        let standard_environment = standard.environment_for_modules(standard_module_names);
        if application_module_is_empty(&application_module) {
            return standard_environment.as_ref().clone();
        }
        #[cfg(test)]
        standard_reuse_counters::record_application_prepare();
        Self::from_module_with_base(&application_module, Some(standard_environment.as_ref()))
    }

    #[cfg(test)]
    pub(crate) fn standard_function_modules_for_test(&self) -> BTreeSet<String> {
        self.functions
            .iter()
            .filter_map(|function| function.module_name.clone())
            .filter(|module| is_standard_module_name(Some(module.as_str())))
            .collect()
    }

    fn from_module_with_base(module: &SurfaceModule, base: Option<&TypeEnvironment>) -> Self {
        facts::from_module_with_base(module, base)
    }

    pub(crate) fn function(&self, name: &str) -> Option<&FunctionSignature> {
        self.functions_named(name).next()
    }

    #[cfg(test)]
    pub(crate) fn function_is_published_for_test(&self, name: &str) -> bool {
        self.functions_named(name).any(|function| {
            !self
                .unpublished_functions
                .contains(&function_publication_key(function))
        })
    }

    pub(in crate::types) fn published_functions(&self) -> impl Iterator<Item = &FunctionSignature> {
        self.functions.iter().filter(|function| {
            !self
                .unpublished_functions
                .contains(&function_publication_key(function))
        })
    }

    pub(crate) fn function_is_published(&self, function: &Function) -> bool {
        !self
            .unpublished_functions
            .contains(&source_function_publication_key(function))
    }

    #[cfg(test)]
    pub(crate) fn local_function_value_recovery(
        &self,
        name: &str,
        current_module: Option<&str>,
    ) -> Option<&FunctionSignature> {
        self.local_function_value_recovery_in_owner(name, current_module, None)
    }

    pub(crate) fn local_function_value_recovery_in_owner(
        &self,
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<&FunctionSignature> {
        if self.local_value_recovery_candidate_count_in_owner(name, current_module, current_package)
            != 1
        {
            return None;
        }
        let mut matches = self
            .function_recovery_signatures
            .iter()
            .filter(|signature| {
                signature.module_name.as_deref() == current_module
                    && signature.package_name.as_deref() == current_package
                    && signature.name == name
            });
        let first = matches.next()?;
        matches.next().is_none().then_some(first)
    }

    pub(crate) fn local_value_recovery_candidate_count_in_owner(
        &self,
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> usize {
        self.local_function_value_recovery_count(name, current_module, current_package)
            + self.local_constructor_recovery_count(name, current_module, current_package, None)
    }

    fn local_function_value_recovery_count(
        &self,
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> usize {
        self.function_recovery_signatures
            .iter()
            .filter(|signature| {
                signature.module_name.as_deref() == current_module
                    && signature.package_name.as_deref() == current_package
                    && signature.name == name
            })
            .count()
    }

    #[cfg(test)]
    pub(crate) fn local_call_recovery_candidate_count(
        &self,
        name: &str,
        current_module: Option<&str>,
        arg_count: usize,
    ) -> usize {
        self.local_call_recovery_candidate_count_in_owner(name, current_module, None, arg_count)
    }

    pub(crate) fn local_call_recovery_candidate_count_in_owner(
        &self,
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
        arg_count: usize,
    ) -> usize {
        self.local_function_call_recovery_count(name, current_module, current_package, arg_count)
            + self.local_constructor_recovery_count(
                name,
                current_module,
                current_package,
                Some(arg_count),
            )
    }

    fn local_function_call_recovery_count(
        &self,
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
        arg_count: usize,
    ) -> usize {
        self.function_recoveries
            .iter()
            .filter(|(key, _)| {
                key.module_name.as_deref() == current_module
                    && key.package_name.as_deref() == current_package
                    && key.name.as_str() == name
                    && key.accepts_arg_count(arg_count)
            })
            .map(|(_, count)| *count)
            .sum::<usize>()
    }

    fn local_constructor_recovery_count(
        &self,
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
        arg_count: Option<usize>,
    ) -> usize {
        self.constructor_recoveries
            .iter()
            .filter(|(key, _)| {
                key.module_name.as_deref() == current_module
                    && key.package_name.as_deref() == current_package
                    && key.name.as_str() == name
                    && arg_count.is_none_or(|count| key.field_count == count)
            })
            .map(|(_, count)| *count)
            .sum::<usize>()
    }

    pub(crate) fn function_for(&self, source: &Function) -> Option<&FunctionSignature> {
        let name = source.name.as_deref()?;
        self.functions_named(name).find(|function| {
            function.node_id == source.node_id
                && function.name == name
                && function.module_name == source.module_name
                && function.package_name == source.package_name
                && function.span == source.span
        })
    }

    #[cfg(test)]
    pub(crate) fn unqualified_function(
        &self,
        name: &str,
        current_module: Option<&str>,
    ) -> FunctionLookup<'_> {
        self.unqualified_function_in_owner(name, current_module, None)
    }

    pub(crate) fn unqualified_function_in_owner(
        &self,
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> FunctionLookup<'_> {
        if let Some(function) = self.functions_named(name).find(|function| {
            function.name == name
                && function.module_name.as_deref() == current_module
                && function.package_name.as_deref() == current_package
        }) {
            return FunctionLookup::Found(function);
        }

        let mut matches = self.functions_named(name).filter(|function| {
            self.function_is_unqualified_import(function, name, current_module, current_package)
        });
        let Some(first) = matches.next() else {
            return FunctionLookup::Missing;
        };
        if matches.next().is_some() {
            FunctionLookup::Ambiguous
        } else {
            FunctionLookup::Found(first)
        }
    }

    pub(crate) fn unqualified_function_import_candidates(
        &self,
        name: &str,
        current_module: Option<&str>,
    ) -> Vec<&FunctionSignature> {
        self.functions_named(name)
            .filter(|function| {
                self.function_is_unqualified_import(function, name, current_module, None)
            })
            .collect()
    }

    fn function_is_unqualified_import(
        &self,
        function: &FunctionSignature,
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> bool {
        function.name == name
            && function.visibility == Visibility::Public
            && function.module_name.as_deref().is_some_and(|module_name| {
                self.uses.iter().any(|use_decl| {
                    crate::name_recovery::use_decl_contributes_bare_exports(use_decl)
                        && use_decl.module_name.as_deref() == current_module
                        && current_package.is_none_or(|package| {
                            use_decl.importing_package.as_deref() == Some(package)
                        })
                        && use_decl.name.as_str() == module_name
                        && function.package_name == use_decl.resolved_target().package
                })
            })
    }

    pub(crate) fn function_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<&FunctionSignature> {
        self.function_path_with_inferred_owner(segments, current_module, true)
    }

    pub(crate) fn function_path_in_owner(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<&FunctionSignature> {
        self.function_path_with_companion_access(segments, current_module, current_package, true)
    }

    pub(crate) fn function_path_for_value(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<&FunctionSignature> {
        self.function_path_with_inferred_owner(segments, current_module, false)
    }

    pub(crate) fn function_path_for_value_in_owner(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<&FunctionSignature> {
        self.function_path_with_companion_access(segments, current_module, current_package, false)
    }

    fn function_path_with_inferred_owner(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        allow_companion_private_access: bool,
    ) -> Option<&FunctionSignature> {
        match segments {
            [name] => self.function(name),
            [_, .., name] => {
                let use_decl = imported_use_for_path(
                    &self.uses,
                    &segments[..segments.len() - 1],
                    current_module,
                )?;
                let module_name = use_decl.name.as_str();
                self.functions_named(name).find(|function| {
                    function.module_name.as_deref() == Some(module_name)
                        && self.imported_function_is_visible(
                            function,
                            use_decl,
                            current_module,
                            allow_companion_private_access,
                        )
                })
            }
            _ => None,
        }
    }

    fn function_path_with_companion_access(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
        allow_companion_private_access: bool,
    ) -> Option<&FunctionSignature> {
        match segments {
            [name] => self
                .unqualified_function_in_owner(name, current_module, current_package)
                .found(),
            [_, .., name] => {
                let use_decl = imported_use_for_path_in_owner(
                    &self.uses,
                    &segments[..segments.len() - 1],
                    current_module,
                    current_package,
                )?;
                let module_name = use_decl.name.as_str();
                self.functions_named(name).find(|function| {
                    function.module_name.as_deref() == Some(module_name)
                        && self.imported_function_is_visible(
                            function,
                            use_decl,
                            current_module,
                            allow_companion_private_access,
                        )
                })
            }
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn schema_decode_step_signature(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
    ) -> Option<&FunctionSignature> {
        self.schema_decode_step_signature_in_owner(schema_path, current_module, None)
    }

    pub(crate) fn schema_decode_step_signature_in_owner(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<&FunctionSignature> {
        self.schema_helper_signature(
            schema_path,
            current_module,
            current_package,
            schema_decode_step_function_name,
        )
    }

    pub(crate) fn schema_encode_signature_in_owner(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<&FunctionSignature> {
        self.schema_helper_signature(
            schema_path,
            current_module,
            current_package,
            schema_encode_function_name,
        )
    }

    pub(crate) fn schema_runtime_validation_signature_in_owner(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<&FunctionSignature> {
        self.schema_helper_signature(
            schema_path,
            current_module,
            current_package,
            schema_runtime_validation_function_name,
        )
    }

    fn schema_helper_signature(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
        helper_name_for: fn(&str) -> String,
    ) -> Option<&FunctionSignature> {
        let schema = self.schema_symbols.schema_target_path(
            schema_path,
            current_module,
            current_package,
            &self.uses,
            true,
            &self.companion_schema_access_targets,
            &mut Vec::new(),
        )?;
        let helper_name = helper_name_for(&schema.name);
        self.functions_named(&helper_name).find(|function| {
            function.module_name == schema.module_name
                && function.package_name == schema.package_name
                && self.schema_helper_is_visible(
                    function.visibility,
                    schema.module_name.as_deref(),
                    current_module,
                )
        })
    }

    pub(crate) fn unsupported_schema_encode_field(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<UnsupportedSchemaEncodeField> {
        let schema = self.schema_symbols.schema_target_path(
            schema_path,
            current_module,
            current_package,
            &self.uses,
            true,
            &self.companion_schema_access_targets,
            &mut Vec::new(),
        )?;
        let field = schema.unsupported_format_neutral_encode_field.clone()?;
        Some(UnsupportedSchemaEncodeField {
            schema_name: schema.name.clone(),
            schema_span: schema.span.clone(),
            field,
        })
    }

    #[cfg(test)]
    pub(crate) fn schema_reference_error(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
    ) -> SchemaReferenceError {
        self.schema_reference_error_in_owner(schema_path, current_module, None)
    }

    pub(crate) fn schema_reference_error_in_owner(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> SchemaReferenceError {
        if self.schema_symbols.schema_path_is_ambiguous(
            schema_path,
            current_module,
            current_package,
        ) {
            return SchemaReferenceError {
                kind: SchemaReferenceErrorKind::Ambiguous,
                resolved_kind: None,
            };
        }
        let wrong_kind =
            self.wrong_schema_reference_kind(schema_path, current_module, current_package);
        let visible_schema = self
            .schema_symbols
            .schema_target_path(
                schema_path,
                current_module,
                current_package,
                &self.uses,
                true,
                &self.companion_schema_access_targets,
                &mut Vec::new(),
            )
            .is_some();
        if visible_schema {
            return SchemaReferenceError {
                kind: SchemaReferenceErrorKind::Unresolved,
                resolved_kind: None,
            };
        }
        if self.schema_symbols.private_schema(
            schema_path,
            current_module,
            current_package,
            &self.uses,
            &self.companion_schema_access_targets,
        ) {
            return SchemaReferenceError {
                kind: SchemaReferenceErrorKind::Private,
                resolved_kind: Some("schema"),
            };
        }
        if let Some(alias_target) = self.schema_symbols.schema_alias_target(
            schema_path,
            current_module,
            current_package,
            &self.uses,
        ) && let Some(kind) = self.wrong_schema_reference_kind(
            &alias_target.target,
            alias_target.module_name.as_deref(),
            alias_target.package_name.as_deref(),
        ) {
            return SchemaReferenceError {
                kind: SchemaReferenceErrorKind::WrongKind,
                resolved_kind: Some(kind),
            };
        }
        if let Some(kind) = wrong_kind {
            return SchemaReferenceError {
                kind: SchemaReferenceErrorKind::WrongKind,
                resolved_kind: Some(kind),
            };
        }
        SchemaReferenceError {
            kind: SchemaReferenceErrorKind::Unresolved,
            resolved_kind: None,
        }
    }

    pub(crate) fn companion_schema_access_target(
        &self,
        current_module: Option<&str>,
    ) -> Option<&str> {
        let current_module = current_module?;
        self.companion_schema_access_targets
            .get(current_module)
            .map(String::as_str)
    }

    fn wrong_schema_reference_kind(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<&'static str> {
        let (name, module_name, package_name) =
            self.resolve_symbol_module(schema_path, current_module, current_package)?;
        if self.type_symbols.iter().any(|symbol| {
            symbol.name == name.as_str()
                && symbol.module_name.as_deref() == module_name.as_deref()
                && symbol.package_name == package_name
                && self.symbol_is_visible(symbol, module_name.as_deref(), current_module)
        }) {
            return Some("type");
        }
        if self.functions_named(&name).any(|function| {
            function.module_name.as_deref() == module_name.as_deref()
                && function.package_name == package_name
                && self.symbol_is_visible(function, module_name.as_deref(), current_module)
        }) {
            return Some("function");
        }
        None
    }

    fn resolve_symbol_module(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<(String, Option<String>, Option<String>)> {
        match segments {
            [name] => Some((
                name.clone(),
                current_module.map(str::to_string),
                current_package.map(str::to_string),
            )),
            [_, .., name] => {
                let use_decl = imported_use_for_path_in_owner(
                    &self.uses,
                    &segments[..segments.len() - 1],
                    current_module,
                    current_package,
                )?;
                Some((
                    name.clone(),
                    Some(use_decl.name.clone()),
                    use_decl.resolved_target().package,
                ))
            }
            _ => None,
        }
    }

    fn symbol_is_visible(
        &self,
        symbol: &impl SymbolVisibility,
        module_name: Option<&str>,
        current_module: Option<&str>,
    ) -> bool {
        module_name == current_module || symbol.visibility() == Visibility::Public
    }

    fn schema_helper_is_visible(
        &self,
        visibility: Visibility,
        schema_module: Option<&str>,
        current_module: Option<&str>,
    ) -> bool {
        schema_module == current_module
            || visibility == Visibility::Public
            || current_module.is_some_and(|current_module| {
                schema_module.is_some_and(|schema_module| {
                    self.companion_schema_access_targets
                        .get(current_module)
                        .is_some_and(|allowed_target| allowed_target == schema_module)
                })
            })
    }

    fn imported_function_is_visible(
        &self,
        function: &FunctionSignature,
        use_decl: &UseDecl,
        current_module: Option<&str>,
        allow_companion_private_access: bool,
    ) -> bool {
        if function.package_name != use_decl.resolved_target().package {
            return false;
        }
        if function.visibility == Visibility::Public {
            return true;
        }
        if use_decl.package.is_some() {
            return false;
        }
        if use_decl.importing_package.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
            && function
                .module_name
                .as_deref()
                .is_some_and(|module| module.starts_with("std::"))
        {
            return true;
        }
        if !allow_companion_private_access {
            return false;
        }
        current_module.is_some_and(|current_module| {
            function.module_name.as_ref().is_some_and(|target_module| {
                self.companion_function_access_targets
                    .get(current_module)
                    .is_some_and(|allowed_target| allowed_target == target_module)
            })
        })
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct FunctionRecoveryKey {
    package_name: Option<String>,
    module_name: Option<String>,
    name: String,
    fixed_arg_count: usize,
    has_variadic: bool,
}

impl FunctionRecoveryKey {
    fn accepts_arg_count(&self, arg_count: usize) -> bool {
        if self.has_variadic {
            arg_count >= self.fixed_arg_count
        } else {
            arg_count == self.fixed_arg_count
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ConstructorRecoveryKey {
    package_name: Option<String>,
    module_name: Option<String>,
    name: String,
    field_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ImportConstructorRecoveryKey {
    current_package: Option<String>,
    current_module: Option<String>,
    target_package: Option<String>,
    target_module: String,
    alias: String,
    constructor_segments: Vec<String>,
    field_count: usize,
}
