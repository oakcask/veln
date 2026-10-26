use super::super::signature_collection::{
    canonicalize_effect_signature_types, effect_signatures, handler_signatures,
    ordinary_function_signatures,
};
use super::*;

use crate::type_syntax::parse_type_or_unknown;

mod quarantine;

use quarantine::{
    quarantine_invalid_variant_refinement_declaration_types,
    quarantine_invalid_variant_refinement_function_types,
    quarantine_invalid_variant_refinement_handler_types,
    quarantine_invalid_variant_refinement_signatures,
};

trait BaseFacts {
    fn extend_with(&mut self, base: &Self);
}

impl<T: Clone> BaseFacts for Vec<T> {
    fn extend_with(&mut self, base: &Self) {
        self.extend(base.clone());
    }
}

impl<K: Clone + Ord, V: Clone> BaseFacts for BTreeMap<K, V> {
    fn extend_with(&mut self, base: &Self) {
        self.extend(base.clone());
    }
}

impl BaseFacts for SchemaSymbolTable {
    fn extend_with(&mut self, base: &Self) {
        self.extend(base.clone());
    }
}

struct DeclarationFacts {
    effects: Vec<EffectSignature>,
    adts: AdtRegistry,
    companion_effect_access_targets: BTreeMap<String, CompanionAccessTarget>,
}

struct CallableFacts {
    functions: Vec<FunctionSignature>,
    handlers: Vec<HandlerSignature>,
}

struct SymbolFacts {
    schema_symbols: SchemaSymbolTable,
    type_symbols: Vec<NamedSymbol>,
    uses: Vec<UseDecl>,
    quarantined_uses: Vec<UseDecl>,
    companion_function_access_targets: BTreeMap<String, String>,
    companion_schema_access_targets: BTreeMap<String, String>,
}

#[derive(Default)]
struct RecoveryFacts {
    function_signatures: Vec<FunctionSignature>,
    functions: BTreeMap<FunctionRecoveryKey, usize>,
    constructors: BTreeMap<ConstructorRecoveryKey, usize>,
    import_constructors: BTreeMap<ImportConstructorRecoveryKey, usize>,
}

impl RecoveryFacts {
    fn record_function(&mut self, signature: FunctionSignature) {
        *self
            .functions
            .entry(FunctionRecoveryKey {
                module_name: signature.module_name.clone(),
                name: signature.name.clone(),
                fixed_arg_count: signature.params.len(),
                has_variadic: signature.variadic.is_some(),
            })
            .or_insert(0) += 1;
        self.function_signatures.push(signature);
    }
}

pub(super) fn from_module_with_base(
    module: &SurfaceModule,
    base: Option<&TypeEnvironment>,
) -> TypeEnvironment {
    let mut declarations = declaration_facts(module, base);
    let validation_environment = TypeEnvironment::for_path_classification_with_base(module, base);
    quarantine_invalid_variant_refinement_declaration_types(
        module,
        &validation_environment,
        &mut declarations.effects,
        &mut declarations.adts,
    );
    let callables = callable_facts(module, base, &declarations);
    finish_environment(module, base, declarations, callables)
}

pub(super) fn from_module_for_path_classification(module: &SurfaceModule) -> TypeEnvironment {
    from_module_for_path_classification_with_base(module, None)
}

pub(super) fn from_module_for_path_classification_with_base(
    module: &SurfaceModule,
    base: Option<&TypeEnvironment>,
) -> TypeEnvironment {
    let declarations = declaration_facts(module, base);
    let mut callables = declared_callable_facts(module, base, &declarations);
    append_schema_helpers(module, &mut callables.functions, &declarations.adts);
    finish_environment(module, base, declarations, callables)
}

fn finish_environment(
    module: &SurfaceModule,
    base: Option<&TypeEnvironment>,
    declarations: DeclarationFacts,
    callables: CallableFacts,
) -> TypeEnvironment {
    let symbols = symbol_facts(module, base, &declarations.adts);
    let mut environment = initial_environment(module, base, declarations, callables, symbols);
    quarantine_invalid_variant_refinement_signatures(module, &mut environment);
    finish_function_facts(module, &mut environment);
    finish_module_value_facts(module, &mut environment);
    install_recovery_facts(module, &mut environment);
    environment
}

fn initial_environment(
    module: &SurfaceModule,
    base: Option<&TypeEnvironment>,
    declarations: DeclarationFacts,
    callables: CallableFacts,
    symbols: SymbolFacts,
) -> TypeEnvironment {
    TypeEnvironment {
        module_values: Vec::new(),
        module_values_by_identity: HashMap::new(),
        functions: callables.functions,
        functions_by_name: HashMap::new(),
        unpublished_functions: base
            .map(|base| base.unpublished_functions.clone())
            .unwrap_or_default(),
        function_recovery_signatures: Vec::new(),
        function_recoveries: BTreeMap::new(),
        constructor_recoveries: BTreeMap::new(),
        import_constructor_recoveries: BTreeMap::new(),
        effects: declarations.effects,
        handlers: callables.handlers,
        schema_symbols: symbols.schema_symbols,
        type_symbols: symbols.type_symbols,
        uses: symbols.uses,
        quarantined_uses: symbols.quarantined_uses,
        invalid_names: module.invalid_names.clone(),
        adts: declarations.adts,
        companion_function_access_targets: symbols.companion_function_access_targets,
        companion_schema_access_targets: symbols.companion_schema_access_targets,
        companion_effect_access_targets: declarations.companion_effect_access_targets,
    }
}

fn finish_function_facts(module: &SurfaceModule, environment: &mut TypeEnvironment) {
    let unpublished = module
        .functions
        .iter()
        .filter(|function| {
            crate::analysis::function_has_private_variant_refinement_exposure(function, environment)
                || crate::analysis::function_depends_on_quarantined_adt(function, environment)
        })
        .map(source_function_publication_key)
        .collect::<Vec<_>>();
    environment.unpublished_functions.extend(unpublished);
    let aliases =
        function_alias_signatures_with_target_filter(module, &environment.functions, |target| {
            !environment
                .unpublished_functions
                .contains(&function_publication_key(target))
        });
    environment.functions.extend(aliases);
    environment.functions_by_name = function_name_index(&environment.functions);
}

fn finish_module_value_facts(module: &SurfaceModule, environment: &mut TypeEnvironment) {
    environment.module_values = module
        .module_values
        .iter()
        .filter(|value| value.name.is_some())
        .map(|value| ModuleValueSignature {
            declaration: value.clone(),
            ty: parse_type_or_unknown(Some(&value.annotation)),
            available: true,
        })
        .collect();
    let module_value_types = environment
        .module_values
        .iter()
        .map(|value| {
            environment.canonicalize_type_annotation(
                value.ty.clone(),
                value.declaration.module_name.as_deref(),
            )
        })
        .collect::<Vec<_>>();
    for (value, ty) in environment.module_values.iter_mut().zip(module_value_types) {
        value.ty = ty;
    }
    for (index, value) in environment.module_values.iter().enumerate() {
        if let Some(name) = &value.declaration.name {
            let identity = (value.declaration.module_name.clone(), name.clone());
            environment
                .module_values_by_identity
                .entry(identity)
                .and_modify(|entry| *entry = ModuleValueIndex::Collision)
                .or_insert(ModuleValueIndex::Unique(index));
        }
    }
}

fn install_recovery_facts(module: &SurfaceModule, environment: &mut TypeEnvironment) {
    let recovery = recovery_facts(module, environment);
    environment.function_recovery_signatures = recovery.function_signatures;
    environment.function_recoveries = recovery.functions;
    environment.constructor_recoveries = recovery.constructors;
    environment.import_constructor_recoveries = recovery.import_constructors;
}

fn recovery_facts(module: &SurfaceModule, environment: &TypeEnvironment) -> RecoveryFacts {
    let mut facts = function_recovery_facts(module, environment);
    facts.constructors = constructor_recovery_facts(module);
    facts.import_constructors = import_constructor_recovery_facts(module);
    facts
}

fn function_recovery_facts(module: &SurfaceModule, environment: &TypeEnvironment) -> RecoveryFacts {
    let mut facts = RecoveryFacts::default();
    for function in &module.functions {
        if let Some(signature) = invalid_function_recovery_signature(module, function, environment)
        {
            facts.record_function(signature);
        }
    }
    let no_access_targets = BTreeMap::new();
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
    {
        if let Some(signature) = invalid_alias_recovery_signature(
            module,
            alias,
            &environment.functions,
            &no_access_targets,
        ) {
            facts.record_function(signature);
        }
    }
    facts
}

fn invalid_function_recovery_signature(
    module: &SurfaceModule,
    function: &Function,
    environment: &TypeEnvironment,
) -> Option<FunctionSignature> {
    let name = function.name.as_ref()?;
    if function.kind != FunctionKind::Function
        || !module_has_invalid_name(module, veln_ast::NameClass::Function, name, &function.span)
    {
        return None;
    }
    let (params, variadic) = function_signature_params(function);
    let mut signature = FunctionSignature {
        name: name.clone(),
        target_name: name.clone(),
        module_name: function.module_name.clone(),
        visibility: function.visibility,
        params,
        variadic,
        return_type: parse_type_or_unknown(function.return_type.as_deref()),
        effects: function.effects.clone().unwrap_or_default(),
        callsite: function.callsite.is_some(),
        node_id: function.node_id,
        span: function.span.clone(),
    };
    if quarantine::function_has_invalid_variant_refinement(module, function, environment) {
        signature.params.fill(Type::Unknown);
        if let Some(variadic) = &mut signature.variadic {
            *variadic = Type::Unknown;
        }
        signature.return_type = Type::Unknown;
    }
    Some(signature)
}

fn invalid_alias_recovery_signature(
    module: &SurfaceModule,
    alias: &PublicAlias,
    functions: &[FunctionSignature],
    access_targets: &BTreeMap<String, String>,
) -> Option<FunctionSignature> {
    let name = alias.name.as_ref()?;
    if !module_has_invalid_name(module, veln_ast::NameClass::Function, name, &alias.span) {
        return None;
    }
    let target = function_signature_path(
        &alias.target,
        &module.uses,
        functions,
        alias.module_name.as_deref(),
        access_targets,
    )?;
    Some(FunctionSignature {
        name: name.clone(),
        target_name: target.target_name.clone(),
        module_name: alias.module_name.clone(),
        visibility: Visibility::Public,
        params: target.params.clone(),
        variadic: target.variadic.clone(),
        return_type: target.return_type.clone(),
        effects: target.effects.clone(),
        callsite: target.callsite,
        node_id: alias.node_id,
        span: alias.span.clone(),
    })
}

fn constructor_recovery_facts(module: &SurfaceModule) -> BTreeMap<ConstructorRecoveryKey, usize> {
    let mut candidates = BTreeMap::new();
    for type_decl in &module.types {
        let type_is_invalid = type_decl.name.as_deref().is_some_and(|name| {
            module_has_invalid_name(module, veln_ast::NameClass::Type, name, &type_decl.span)
        });
        for variant in &type_decl.variants {
            let Some(name) = &variant.name else {
                continue;
            };
            let constructor_is_invalid = module_has_invalid_name(
                module,
                veln_ast::NameClass::Constructor,
                name,
                &variant.span,
            );
            if type_is_invalid || constructor_is_invalid {
                *candidates
                    .entry(ConstructorRecoveryKey {
                        module_name: type_decl.module_name.clone(),
                        name: name.clone(),
                        field_count: variant.fields.len(),
                    })
                    .or_insert(0) += 1;
            }
        }
    }
    candidates
}

fn import_constructor_recovery_facts(
    module: &SurfaceModule,
) -> BTreeMap<ImportConstructorRecoveryKey, usize> {
    let mut candidates = BTreeMap::new();
    for use_decl in module
        .uses
        .iter()
        .filter(|use_decl| crate::name_recovery::use_decl_is_recovery_candidate(module, use_decl))
    {
        for type_decl in module.types.iter().filter(|type_decl| {
            type_decl.module_name.as_deref() == Some(use_decl.name.as_str())
                && type_decl.visibility == Visibility::Public
        }) {
            let Some(type_name) = &type_decl.name else {
                continue;
            };
            for variant in type_decl
                .variants
                .iter()
                .filter(|variant| variant.visibility == Visibility::Public)
            {
                let Some(variant_name) = &variant.name else {
                    continue;
                };
                for constructor_segments in [
                    vec![variant_name.clone()],
                    vec![type_name.clone(), variant_name.clone()],
                ] {
                    *candidates
                        .entry(ImportConstructorRecoveryKey {
                            current_module: use_decl.module_name.clone(),
                            alias: use_decl.alias.clone(),
                            constructor_segments,
                            field_count: variant.fields.len(),
                        })
                        .or_insert(0) += 1;
                }
            }
        }
    }
    candidates
}

fn module_has_invalid_name(
    module: &SurfaceModule,
    class: veln_ast::NameClass,
    name: &str,
    container: &veln_source::SourceSpan,
) -> bool {
    module.invalid_names.iter().any(|invalid| {
        invalid.class == class
            && invalid.name == name
            && invalid.span.file == container.file
            && container.start.offset <= invalid.span.start.offset
            && invalid.span.end.offset <= container.end.offset
    })
}

pub(super) fn function_name_index(functions: &[FunctionSignature]) -> HashMap<String, Vec<usize>> {
    let mut index = HashMap::<String, Vec<usize>>::new();
    for (position, function) in functions.iter().enumerate() {
        index
            .entry(function.name.clone())
            .or_default()
            .push(position);
    }
    index
}

fn declaration_facts(module: &SurfaceModule, base: Option<&TypeEnvironment>) -> DeclarationFacts {
    let mut effects = effect_signatures(module);
    extend_with_base_facts(&mut effects, base.map(|base| &base.effects));
    let adts = if let Some(base) = base {
        AdtRegistry::from_module_with_base(module, &base.adts)
    } else {
        AdtRegistry::from_module(module)
    };
    let mut companion_effect_access_targets = companion_access_target_infos(module);
    extend_with_base_facts(
        &mut companion_effect_access_targets,
        base.map(|base| &base.companion_effect_access_targets),
    );
    canonicalize_effect_signature_types(
        module,
        &mut effects,
        &adts,
        &companion_effect_access_targets,
    );
    DeclarationFacts {
        effects,
        adts,
        companion_effect_access_targets,
    }
}

fn declared_callable_facts(
    module: &SurfaceModule,
    base: Option<&TypeEnvironment>,
    declarations: &DeclarationFacts,
) -> CallableFacts {
    let mut handlers = handler_signatures(
        module,
        &declarations.effects,
        &declarations.adts,
        &declarations.companion_effect_access_targets,
    );
    extend_with_base_facts(&mut handlers, base.map(|base| &base.handlers));
    let mut functions = ordinary_function_signatures(
        module,
        &declarations.effects,
        &declarations.adts,
        &declarations.companion_effect_access_targets,
    );
    if let Some(base) = base {
        functions.extend(base.published_functions().cloned());
    }
    CallableFacts {
        functions,
        handlers,
    }
}

fn callable_facts(
    module: &SurfaceModule,
    base: Option<&TypeEnvironment>,
    declarations: &DeclarationFacts,
) -> CallableFacts {
    let CallableFacts {
        mut functions,
        mut handlers,
    } = declared_callable_facts(module, base, declarations);
    let validation_environment = TypeEnvironment::for_path_classification_with_base(module, base);
    quarantine_invalid_variant_refinement_function_types(
        module,
        &validation_environment,
        &mut functions,
    );
    quarantine_invalid_variant_refinement_handler_types(
        module,
        &validation_environment,
        &mut handlers,
    );
    infer_private_function_body_return_types(module, &mut functions, &declarations.adts);
    infer_private_function_call_site_signature_types(module, &mut functions, &declarations.adts);
    infer_private_function_body_return_types(module, &mut functions, &declarations.adts);
    infer_private_prelude_callback_return_types(module, &mut functions, &declarations.adts);
    append_schema_helpers(module, &mut functions, &declarations.adts);
    infer_function_and_private_handler_effects(
        module,
        &mut functions,
        &declarations.effects,
        &mut handlers,
    );
    CallableFacts {
        functions,
        handlers,
    }
}

fn append_schema_helpers(
    module: &SurfaceModule,
    functions: &mut Vec<FunctionSignature>,
    adts: &AdtRegistry,
) {
    functions.extend(schema_decode_function_signatures(module, adts));
    functions.extend(schema_encode_function_signatures(module, adts));
    functions.extend(schema_validate_function_signatures(module));
}

fn symbol_facts(
    module: &SurfaceModule,
    base: Option<&TypeEnvironment>,
    adts: &AdtRegistry,
) -> SymbolFacts {
    let mut schema_symbols = SchemaSymbolTable::from_module(module, adts);
    extend_with_base_facts(&mut schema_symbols, base.map(|base| &base.schema_symbols));
    let mut type_symbols = named_type_symbols(module);
    extend_with_base_facts(&mut type_symbols, base.map(|base| &base.type_symbols));
    let mut uses = crate::name_recovery::normal_use_decls(module);
    extend_with_base_facts(&mut uses, base.map(|base| &base.uses));
    let mut quarantined_uses = module
        .uses
        .iter()
        .filter(|use_decl| crate::name_recovery::use_decl_is_recovery_candidate(module, use_decl))
        .cloned()
        .collect();
    extend_with_base_facts(
        &mut quarantined_uses,
        base.map(|base| &base.quarantined_uses),
    );
    let mut companion_function_access_targets = companion_function_access_targets(module);
    extend_with_base_facts(
        &mut companion_function_access_targets,
        base.map(|base| &base.companion_function_access_targets),
    );
    let mut companion_schema_access_targets = companion_access_targets(module);
    extend_with_base_facts(
        &mut companion_schema_access_targets,
        base.map(|base| &base.companion_schema_access_targets),
    );
    SymbolFacts {
        schema_symbols,
        type_symbols,
        uses,
        quarantined_uses,
        companion_function_access_targets,
        companion_schema_access_targets,
    }
}

fn extend_with_base_facts<T: BaseFacts>(facts: &mut T, base: Option<&T>) {
    if let Some(base) = base {
        facts.extend_with(base);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extending_with_base_facts_preserves_collection_merge_order() {
        let mut functions = vec!["application"];
        let base_functions = vec!["standard"];
        extend_with_base_facts(&mut functions, Some(&base_functions));
        assert_eq!(functions, ["application", "standard"]);

        let mut targets = BTreeMap::from([("shared", "application"), ("local", "application")]);
        let base_targets = BTreeMap::from([("shared", "standard"), ("standard", "standard")]);
        extend_with_base_facts(&mut targets, Some(&base_targets));
        assert_eq!(targets["shared"], "standard");
        assert_eq!(targets["local"], "application");
        assert_eq!(targets["standard"], "standard");
    }
}
