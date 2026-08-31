use super::*;
use veln_ast::NameOccurrence;

mod effect_handlers;
mod facts;
mod quarantined_imports;
mod standard_subset;

#[derive(Clone)]
pub(crate) struct TypeEnvironment {
    functions: Vec<FunctionSignature>,
    functions_by_name: HashMap<String, Vec<usize>>,
    function_recovery_signatures: Vec<FunctionSignature>,
    function_recoveries: BTreeMap<FunctionRecoveryKey, usize>,
    constructor_recoveries: BTreeMap<ConstructorRecoveryKey, usize>,
    import_constructor_recoveries: BTreeMap<ImportConstructorRecoveryKey, usize>,
    codec_calls: Vec<CodecCallSignature>,
    effects: Vec<EffectSignature>,
    handlers: Vec<HandlerSignature>,
    schema_symbols: SchemaSymbolTable,
    type_symbols: Vec<NamedSymbol>,
    codec_symbols: Vec<NamedSymbol>,
    pub(crate) uses: Vec<UseDecl>,
    quarantined_uses: Vec<UseDecl>,
    invalid_names: Vec<InvalidName>,
    pub(crate) adts: AdtRegistry,
    companion_function_access_targets: BTreeMap<String, String>,
    companion_schema_access_targets: BTreeMap<String, String>,
    companion_effect_access_targets: BTreeMap<String, CompanionAccessTarget>,
}

impl TypeEnvironment {
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

    pub(crate) fn local_function_value_recovery(
        &self,
        name: &str,
        current_module: Option<&str>,
    ) -> Option<&FunctionSignature> {
        if self.local_value_recovery_candidate_count(name, current_module) != 1 {
            return None;
        }
        let mut matches = self
            .function_recovery_signatures
            .iter()
            .filter(|signature| {
                signature.module_name.as_deref() == current_module && signature.name == name
            });
        let first = matches.next()?;
        matches.next().is_none().then_some(first)
    }

    pub(crate) fn local_value_recovery_candidate_count(
        &self,
        name: &str,
        current_module: Option<&str>,
    ) -> usize {
        self.local_function_value_recovery_count(name, current_module)
            + self.local_constructor_recovery_count(name, current_module, None)
    }

    fn local_function_value_recovery_count(
        &self,
        name: &str,
        current_module: Option<&str>,
    ) -> usize {
        self.function_recovery_signatures
            .iter()
            .filter(|signature| {
                signature.module_name.as_deref() == current_module && signature.name == name
            })
            .count()
    }

    pub(crate) fn local_call_recovery_candidate_count(
        &self,
        name: &str,
        current_module: Option<&str>,
        arg_count: usize,
    ) -> usize {
        self.local_function_call_recovery_count(name, current_module, arg_count)
            + self.local_constructor_recovery_count(name, current_module, Some(arg_count))
    }

    fn local_function_call_recovery_count(
        &self,
        name: &str,
        current_module: Option<&str>,
        arg_count: usize,
    ) -> usize {
        self.function_recoveries
            .iter()
            .filter(|(key, _)| {
                key.module_name.as_deref() == current_module
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
        arg_count: Option<usize>,
    ) -> usize {
        self.constructor_recoveries
            .iter()
            .filter(|(key, _)| {
                key.module_name.as_deref() == current_module
                    && key.name.as_str() == name
                    && arg_count.is_none_or(|count| key.field_count == count)
            })
            .map(|(_, count)| *count)
            .sum::<usize>()
    }

    pub(crate) fn canonicalize_type_annotation(
        &self,
        ty: Type,
        current_module: Option<&str>,
    ) -> Type {
        canonicalize_type_effects(
            ty,
            &self.uses,
            &self.quarantined_uses,
            current_module,
            &self.effects,
            &self.adts,
            &self.companion_effect_access_targets,
        )
    }

    pub(crate) fn function_for(&self, source: &Function) -> Option<&FunctionSignature> {
        let name = source.name.as_deref()?;
        self.functions_named(name).find(|function| {
            function.node_id == source.node_id
                && function.name == name
                && function.module_name == source.module_name
                && function.span == source.span
        })
    }

    pub(crate) fn unqualified_function(
        &self,
        name: &str,
        current_module: Option<&str>,
    ) -> FunctionLookup<'_> {
        if let Some(function) = self.functions_named(name).find(|function| {
            function.name == name && function.module_name.as_deref() == current_module
        }) {
            return FunctionLookup::Found(function);
        }

        let mut matches = self
            .functions_named(name)
            .filter(|function| self.function_is_unqualified_import(function, name, current_module));
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
            .filter(|function| self.function_is_unqualified_import(function, name, current_module))
            .collect()
    }

    fn function_is_unqualified_import(
        &self,
        function: &FunctionSignature,
        name: &str,
        current_module: Option<&str>,
    ) -> bool {
        function.name == name
            && function.visibility == Visibility::Public
            && function.module_name.as_deref().is_some_and(|module_name| {
                self.uses.iter().any(|use_decl| {
                    use_decl.module_name.as_deref() == current_module
                        && use_decl.name.as_str() == module_name
                })
            })
    }

    pub(crate) fn function_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<&FunctionSignature> {
        self.function_path_with_companion_access(segments, current_module, true)
    }

    pub(crate) fn function_path_for_value(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<&FunctionSignature> {
        self.function_path_with_companion_access(segments, current_module, false)
    }

    fn function_path_with_companion_access(
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
                        && !self.imported_codec_helper_is_hidden(function, use_decl)
                })
            }
            _ => None,
        }
    }

    pub(crate) fn schema_decode_step_signature(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
    ) -> Option<&FunctionSignature> {
        self.schema_helper_signature(
            schema_path,
            current_module,
            schema_decode_step_function_name,
        )
    }

    pub(crate) fn schema_encode_signature(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
    ) -> Option<&FunctionSignature> {
        self.schema_helper_signature(schema_path, current_module, schema_encode_function_name)
    }

    fn schema_helper_signature(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
        helper_name_for: fn(&str) -> String,
    ) -> Option<&FunctionSignature> {
        let schema = self.schema_symbols.schema_target_path(
            schema_path,
            current_module,
            &self.uses,
            true,
            &self.companion_schema_access_targets,
            &mut Vec::new(),
        )?;
        let helper_name = helper_name_for(&schema.name);
        self.functions_named(&helper_name).find(|function| {
            function.module_name == schema.module_name
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
    ) -> Option<UnsupportedSchemaEncodeField> {
        let schema = self.schema_symbols.schema_target_path(
            schema_path,
            current_module,
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

    pub(crate) fn schema_reference_error(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
    ) -> SchemaReferenceError {
        if self.schema_symbols.private_schema(
            schema_path,
            current_module,
            &self.uses,
            &self.companion_schema_access_targets,
        ) {
            return SchemaReferenceError {
                kind: SchemaReferenceErrorKind::Private,
                resolved_kind: Some("schema"),
            };
        }
        if let Some(alias_target) =
            self.schema_symbols
                .schema_alias_target(schema_path, current_module, &self.uses)
            && let Some(kind) = self.wrong_schema_reference_kind(
                &alias_target.target,
                alias_target.module_name.as_deref(),
            )
        {
            return SchemaReferenceError {
                kind: SchemaReferenceErrorKind::WrongKind,
                resolved_kind: Some(kind),
            };
        }
        if let Some(kind) = self.wrong_schema_reference_kind(schema_path, current_module) {
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
    ) -> Option<&'static str> {
        let (name, module_name) = self.resolve_symbol_module(schema_path, current_module)?;
        if self.type_symbols.iter().any(|symbol| {
            symbol.name == name.as_str()
                && symbol.module_name.as_deref() == module_name.as_deref()
                && self.symbol_is_visible(symbol, module_name.as_deref(), current_module)
        }) {
            return Some("type");
        }
        if self.functions_named(&name).any(|function| {
            function.module_name.as_deref() == module_name.as_deref()
                && self.symbol_is_visible(function, module_name.as_deref(), current_module)
        }) {
            return Some("function");
        }
        if self.codec_symbols.iter().any(|symbol| {
            symbol.name == name.as_str()
                && symbol.module_name.as_deref() == module_name.as_deref()
                && self.symbol_is_visible(symbol, module_name.as_deref(), current_module)
        }) {
            return Some("codec");
        }
        None
    }

    fn resolve_symbol_module(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<(String, Option<String>)> {
        match segments {
            [name] => Some((name.clone(), current_module.map(str::to_string))),
            [_, .., name] => {
                let use_decl = imported_use_for_path(
                    &self.uses,
                    &segments[..segments.len() - 1],
                    current_module,
                )?;
                Some((name.clone(), Some(use_decl.name.clone())))
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

    fn imported_codec_helper_is_hidden(
        &self,
        function: &FunctionSignature,
        use_decl: &UseDecl,
    ) -> bool {
        function.visibility != Visibility::Public
            && self.codec_calls.iter().any(|codec| {
                codec.module_name.as_deref() == Some(use_decl.name.as_str())
                    && codec.target_name == function.target_name
            })
    }

    fn imported_function_is_visible(
        &self,
        function: &FunctionSignature,
        use_decl: &UseDecl,
        current_module: Option<&str>,
        allow_companion_private_access: bool,
    ) -> bool {
        if function.visibility == Visibility::Public {
            return true;
        }
        if use_decl.package.is_some() {
            return false;
        }
        if current_module.is_some_and(|module| module.starts_with("std::"))
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

    pub(crate) fn unqualified_codec_calls(
        &self,
        name: &str,
        current_module: Option<&str>,
    ) -> Vec<&CodecCallSignature> {
        self.codec_calls
            .iter()
            .filter(|codec| codec.name == name && codec.module_name.as_deref() == current_module)
            .collect()
    }

    pub(crate) fn codec_call_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Vec<&CodecCallSignature> {
        match segments {
            [name] => self.unqualified_codec_calls(name, current_module),
            [_, .., name] => {
                let Some(use_decl) = imported_use_for_path(
                    &self.uses,
                    &segments[..segments.len() - 1],
                    current_module,
                ) else {
                    return Vec::new();
                };
                let module_name = use_decl.name.as_str();
                self.codec_calls
                    .iter()
                    .filter(|codec| {
                        codec.name == *name
                            && codec.module_name.as_deref() == Some(module_name)
                            && codec.visibility == Visibility::Public
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct FunctionRecoveryKey {
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
    module_name: Option<String>,
    name: String,
    field_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ImportConstructorRecoveryKey {
    current_module: Option<String>,
    alias: String,
    constructor_segments: Vec<String>,
    field_count: usize,
}
