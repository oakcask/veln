use super::*;

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
    codec_symbols: Vec<NamedSymbol>,
    uses: Vec<UseDecl>,
    companion_function_access_targets: BTreeMap<String, String>,
    companion_schema_access_targets: BTreeMap<String, String>,
}

#[derive(Default)]
struct RecoveryFacts {
    function_signatures: Vec<FunctionSignature>,
    functions: BTreeMap<FunctionRecoveryKey, usize>,
    constructors: BTreeMap<ConstructorRecoveryKey, usize>,
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
    let declarations = declaration_facts(module, base);
    let mut callables = callable_facts(module, base, &declarations);
    let mut codec_calls = codec_facts(module, base, &callables.functions);
    let symbols = symbol_facts(module, base);
    let aliases = function_alias_signatures(module, &callables.functions);
    callables.functions.extend(aliases);
    codec_calls.shrink_to_fit();
    let functions_by_name = function_name_index(&callables.functions);
    let recovery = recovery_facts(module, &callables.functions);

    TypeEnvironment {
        functions: callables.functions,
        functions_by_name,
        function_recovery_signatures: recovery.function_signatures,
        function_recoveries: recovery.functions,
        constructor_recoveries: recovery.constructors,
        codec_calls,
        effects: declarations.effects,
        handlers: callables.handlers,
        schema_symbols: symbols.schema_symbols,
        type_symbols: symbols.type_symbols,
        codec_symbols: symbols.codec_symbols,
        uses: symbols.uses,
        adts: declarations.adts,
        companion_function_access_targets: symbols.companion_function_access_targets,
        companion_schema_access_targets: symbols.companion_schema_access_targets,
        companion_effect_access_targets: declarations.companion_effect_access_targets,
    }
}

fn recovery_facts(module: &SurfaceModule, functions: &[FunctionSignature]) -> RecoveryFacts {
    RecoveryFacts {
        constructors: constructor_recovery_facts(module),
        ..function_recovery_facts(module, functions)
    }
}

fn function_recovery_facts(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> RecoveryFacts {
    let mut facts = RecoveryFacts::default();
    for function in &module.functions {
        if let Some(signature) = invalid_function_recovery_signature(module, function) {
            facts.record_function(signature);
        }
    }
    let no_access_targets = BTreeMap::new();
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
    {
        if let Some(signature) =
            invalid_alias_recovery_signature(module, alias, functions, &no_access_targets)
        {
            facts.record_function(signature);
        }
    }
    facts
}

fn invalid_function_recovery_signature(
    module: &SurfaceModule,
    function: &Function,
) -> Option<FunctionSignature> {
    let name = function.name.as_ref()?;
    if function.kind != FunctionKind::Function
        || !module_has_invalid_name(module, veln_ast::NameClass::Function, name, &function.span)
    {
        return None;
    }
    let (params, variadic) = function_signature_params(function);
    Some(FunctionSignature {
        name: name.clone(),
        target_name: name.clone(),
        module_name: function.module_name.clone(),
        visibility: function.visibility,
        params,
        variadic,
        return_type: parse_type_or_unknown(function.return_type.as_deref()),
        effects: function.effects.clone().unwrap_or_default(),
        node_id: function.node_id,
        span: function.span.clone(),
    })
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
    let adts = AdtRegistry::from_module_with_base(module, base.map(|base| &base.adts));
    let mut companion_effect_access_targets = companion_access_target_infos(module);
    extend_with_base_facts(
        &mut companion_effect_access_targets,
        base.map(|base| &base.companion_effect_access_targets),
    );
    DeclarationFacts {
        effects,
        adts,
        companion_effect_access_targets,
    }
}

fn callable_facts(
    module: &SurfaceModule,
    base: Option<&TypeEnvironment>,
    declarations: &DeclarationFacts,
) -> CallableFacts {
    let mut handlers = handler_signatures(
        module,
        &declarations.effects,
        &declarations.companion_effect_access_targets,
    );
    extend_with_base_facts(&mut handlers, base.map(|base| &base.handlers));
    let mut functions = ordinary_function_signatures(
        module,
        &declarations.effects,
        &declarations.adts,
        &declarations.companion_effect_access_targets,
    );
    extend_with_base_facts(&mut functions, base.map(|base| &base.functions));
    infer_private_function_body_return_types(module, &mut functions, &declarations.adts);
    infer_private_function_call_site_signature_types(module, &mut functions, &declarations.adts);
    infer_private_function_body_return_types(module, &mut functions, &declarations.adts);
    infer_private_prelude_callback_return_types(module, &mut functions, &declarations.adts);
    functions.extend(schema_decode_function_signatures(module));
    functions.extend(schema_encode_function_signatures(module));
    functions.extend(schema_validate_function_signatures(module));
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

fn codec_facts(
    module: &SurfaceModule,
    base: Option<&TypeEnvironment>,
    functions: &[FunctionSignature],
) -> Vec<CodecCallSignature> {
    let mut codec_calls = codec_call_signatures(module, functions);
    extend_with_base_facts(&mut codec_calls, base.map(|base| &base.codec_calls));
    codec_calls
}

fn symbol_facts(module: &SurfaceModule, base: Option<&TypeEnvironment>) -> SymbolFacts {
    let mut schema_symbols = SchemaSymbolTable::from_module(module);
    extend_with_base_facts(&mut schema_symbols, base.map(|base| &base.schema_symbols));
    let mut type_symbols = named_type_symbols(module);
    extend_with_base_facts(&mut type_symbols, base.map(|base| &base.type_symbols));
    let mut codec_symbols = named_codec_symbols(module);
    extend_with_base_facts(&mut codec_symbols, base.map(|base| &base.codec_symbols));
    let mut uses = module.uses.clone();
    extend_with_base_facts(&mut uses, base.map(|base| &base.uses));
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
        codec_symbols,
        uses,
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
