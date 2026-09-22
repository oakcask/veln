impl IndexedDependencies {
    pub(crate) fn empty() -> Self {
        Self {
            files: Vec::new(),
            declarations: FileDeclarations::default(),
            module: empty_surface_module(),
        }
    }

    pub(crate) fn new_direct(dependencies: Vec<DirectDependencySnapshot>) -> Self {
        let mut indexed = Self::index(dependencies);
        attach_classified_path_segments(&mut indexed.files, &indexed.module, &indexed.module);
        indexed
    }

    pub(crate) fn new_standard_library(
        standard_library: Option<DirectDependencySnapshot>,
    ) -> Self {
        let mut indexed = Self::index(standard_library);
        attach_classified_path_segments(&mut indexed.files, &indexed.module, &indexed.module);
        indexed
    }

    fn index(dependencies: impl IntoIterator<Item = DirectDependencySnapshot>) -> Self {
        let mut files = Vec::new();
        let mut declarations = FileDeclarations::default();
        let mut module = empty_surface_module();
        for dependency in dependencies {
            index_dependency_sources(&mut files, &mut declarations, &mut module, dependency);
        }
        Self {
            files,
            declarations,
            module,
        }
    }
}

impl SymbolIndex {
    pub(crate) fn workspace_schema_alias_is_eligible(
        &self,
        module: &str,
        name: &str,
        declaration: &NavigationLocation,
    ) -> bool {
        self.schema_aliases.iter().any(|candidate| {
            candidate.package.is_none()
                && candidate.module == module
                && candidate.name == name
                && candidate.declaration == *declaration
        })
    }

    pub(crate) fn new(
        sources: Vec<SourceFile>,
        direct_dependencies: &IndexedDependencies,
        standard_library: &IndexedDependencies,
    ) -> Self {
        Self::new_with_path_classification(
            sources,
            direct_dependencies,
            standard_library,
            true,
        )
    }

    pub(crate) fn new_for_schema_navigation(
        sources: Vec<SourceFile>,
        direct_dependencies: &IndexedDependencies,
        standard_library: &IndexedDependencies,
    ) -> Self {
        Self::new_with_path_classification(
            sources,
            direct_dependencies,
            standard_library,
            false,
        )
    }

    fn new_with_path_classification(
        sources: Vec<SourceFile>,
        direct_dependencies: &IndexedDependencies,
        standard_library: &IndexedDependencies,
        classify_workspace_paths: bool,
    ) -> Self {
        let mut files = Vec::new();
        let mut declarations = FileDeclarations::default();
        let mut workspace_module = empty_surface_module();
        for source in sources {
            let (file, file_declarations, parsed) = index_workspace_source(source);
            declarations.extend(file_declarations);
            append_parsed_surface_module(&mut workspace_module, &file, &parsed);
            files.push(file);
        }
        declarations.extend(direct_dependencies.declarations.clone());
        declarations.extend(standard_library.declarations.clone());
        if classify_workspace_paths && workspace_needs_path_classification(&files, &workspace_module)
        {
            let mut module = workspace_module.clone();
            append_surface_module(&mut module, direct_dependencies.module.clone());
            append_surface_module(&mut module, standard_library.module.clone());
            attach_classified_path_segments(&mut files, &workspace_module, &module);
        }
        let schema_alias_module_imports = index_schema_alias_module_imports(&files);
        let schema_alias_declarations = declarations
            .schema_aliases
            .iter()
            .chain(&declarations.schema_alias_blockers)
            .cloned()
            .collect::<Vec<_>>();
        let package_schemas = PackageSchemaDeclarations::new(
            &declarations.package_schema_alias_declarations,
            &declarations.package_schema_targets,
            &declarations.recovered_package_schema_targets,
        );
        let schema_aliases = eligible_schema_aliases(
            declarations.schema_aliases,
            &package_schemas,
            &declarations.resolved_package_schema_aliases,
            veln_sema::resolved_schema_aliases(&workspace_module),
        );
        let bare_schema_alias_index = bare_schema_alias_index(
            &declarations.schemas,
            &schema_aliases,
            &schema_alias_declarations,
        );
        let schema_operation_lookup_index = schema_operation_lookup_index(
            &declarations.schemas,
            &schema_aliases,
            &declarations.package_schema_alias_declarations,
        );
        let mut schema_composition_references = workspace_schema_composition_references(
            &files,
            &declarations.schemas,
            &schema_aliases,
            &schema_alias_module_imports,
            veln_sema::resolved_schema_composition_references(&workspace_module),
        );
        let package_schemas = package_schema_index(
            &declarations.schemas,
            &package_schemas,
        );
        schema_composition_references.extend(package_schema_composition_references(
            &files,
            WorkspaceSchemaCompositionDeclarations {
                schemas: &declarations.schemas,
                schema_aliases: &schema_alias_declarations,
                types: &declarations.types,
                type_aliases: &declarations.type_aliases,
            },
            &package_schemas,
            &schema_operation_lookup_index.package_aliases,
            &schema_aliases,
            &schema_alias_module_imports,
        ));
        let type_indices_by_name = symbol_indices_by_name(&declarations.types);
        let type_alias_indices_by_name = symbol_indices_by_name(&declarations.type_aliases);
        let package_type_alias_indices_by_name =
            package_symbol_indices_by_name(&declarations.type_aliases);
        let workspace_type_indices_by_module_and_name =
            workspace_symbol_indices_by_module_and_name(&declarations.types);
        let workspace_type_alias_indices_by_module_and_name =
            workspace_symbol_indices_by_module_and_name(&declarations.type_aliases);
        let package_type_alias_indices_by_module_and_name =
            package_symbol_indices_by_module_and_name(&declarations.type_aliases);
        files.extend(direct_dependencies.files.clone());
        files.extend(standard_library.files.clone());
        Self {
            schemas: declarations.schemas,
            schema_aliases,
            package_schemas,
            effects: declarations.effects,
            handlers: declarations.handlers,
            operations: declarations.operations,
            functions: declarations.functions,
            package_function_targets: declarations.package_function_targets,
            package_type_targets: declarations.package_type_targets,
            package_constructor_targets: declarations.package_constructor_targets,
            types: declarations.types,
            constructors: declarations.constructors,
            type_aliases: declarations.type_aliases,
            type_indices_by_name,
            type_alias_indices_by_name,
            package_type_alias_indices_by_name,
            workspace_type_indices_by_module_and_name,
            workspace_type_alias_indices_by_module_and_name,
            package_type_alias_indices_by_module_and_name,
            schema_composition_references,
            schema_alias_module_imports,
            bare_schema_alias_index,
            schema_operation_lookup_index,
            files,
            function_rename_index: OnceLock::new(),
        }
    }

    fn schema_composition_symbol_at(
        &self,
        file: &IndexedFile,
        token: &Token,
    ) -> Option<Symbol> {
        self.schema_composition_references
            .iter()
            .find(|reference| {
                reference.span.file == *file.source.path()
                    && reference.span.start.offset == token.range.start
                    && reference.span.end.offset == token.range.end
            })
            .map(|reference| match &reference.target {
                SchemaReferenceTarget::Schema(symbol) => Symbol::Schema(symbol.clone()),
                SchemaReferenceTarget::Alias(symbol) => Symbol::SchemaAlias(symbol.clone()),
            })
    }

    fn symbol_at_position(
        self: Arc<Self>,
        source_path: &str,
        position: &SourcePosition,
    ) -> Option<SymbolRequest> {
        let file = self
            .files
            .iter()
            .find(|file| file.source.path().as_str() == source_path)?;
        if file.navigation_isolated {
            return None;
        }
        let offset = offset_for_position(file.source.text(), position)?;
        let tokens = &file.tokens;
        let (token_index, token) = identifier_token_at(tokens, offset)?;
        let selection = file.source.span(token.range);
        let name = file
            .source
            .text()
            .get(selection.start.offset..selection.end.offset)?
            .to_string();
        let selected =
            self.symbol_for_selection(file, tokens, token_index, &name, &selection, None)?;
        let references_supported = !matches!(
            &selected.symbol,
            Symbol::Schema(symbol)
                if matches!(
                    symbol.package_origin,
                    Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
                )
                    && is_schema_operation_path_leaf_candidate_token(tokens, token_index)
                    && !is_schema_operation_path_leaf_token(file, token_index)
        ) && !matches!(
            &selected.symbol,
            Symbol::SchemaAlias(symbol)
                if matches!(
                    symbol.package_origin,
                    Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
                )
                    && is_schema_operation_path_leaf_candidate_token(tokens, token_index)
                    && !is_schema_operation_path_leaf_token(file, token_index)
        );
        Some(SymbolRequest {
            index: self,
            symbol: selected.symbol,
            selection,
            classified_path_segment: selected.classified_path_segment,
            references_supported,
        })
    }

    fn selected_type(&self, result: &NavigationResult) -> Option<TypeSymbol> {
        self.types
            .iter()
            .find(|symbol| {
                symbol.package.is_none()
                    && symbol.declaration == result.selected_symbol.declaration
            })
            .cloned()
            .or_else(|| {
                self.type_aliases
                    .iter()
                    .find(|symbol| {
                        symbol.package.is_none()
                            && symbol.declaration == result.selected_symbol.declaration
                    })
                    .map(|symbol| TypeSymbol {
                        module: symbol.module.clone(),
                        name: symbol.name.clone(),
                        declaration: symbol.declaration.clone(),
                        package: None,
                        package_origin: None,
                        public: true,
                        standard_prelude: symbol.standard_prelude,
                    })
            })
    }

    fn selected_constructor(&self, result: &NavigationResult) -> Option<ConstructorSymbol> {
        self.constructors
            .iter()
            .find(|symbol| {
                symbol.package.is_none()
                    && symbol.declaration == result.selected_symbol.declaration
            })
            .cloned()
    }

    fn selected_function(&self, result: &NavigationResult) -> Option<FunctionSymbol> {
        self.functions
            .iter()
            .find(|symbol| {
                symbol.package.is_none()
                    && symbol.declaration == result.selected_symbol.declaration
            })
            .cloned()
    }

    fn selected_local(&self, result: &NavigationResult) -> Option<LocalSymbol> {
        let NavigationSource::Workspace = result.selected_symbol.declaration.source else {
            return None;
        };
        self.files.iter().find_map(|file| {
            handler_operation_clause_bindings(file, &file.tokens)
                .into_iter()
                .find(|binding| {
                    same_span(
                        &binding.declaration,
                        &result.selected_symbol.declaration.span,
                    )
                })
                .map(|binding| LocalSymbol {
                    name: binding.name,
                    declaration: binding.declaration,
                    scope_file: file.source.path().as_str().to_string(),
                    scope_start: binding.start,
                    scope_end: binding.end,
                    declaration_scope_start: binding.start,
                    declaration_scope_end: binding.end,
                    kind: binding.kind,
                })
        })
    }

    fn affected_spans<'a>(&self, result: &'a NavigationResult) -> Vec<&'a SourceSpan> {
        std::iter::once(&result.definition.span)
            .chain(&result.references)
            .collect()
    }

    fn file_token_for_span<'a>(&'a self, span: &SourceSpan) -> Option<(&'a IndexedFile, usize)> {
        let file = self
            .files
            .iter()
            .find(|file| file.source.path().as_str() == span.file.as_str())?;
        let token_index = file.tokens.iter().position(|token| {
            token.range.start == span.start.offset && token.range.end == span.end.offset
        })?;
        Some((file, token_index))
    }

    fn local_conflict_in_file(
        &self,
        file: &IndexedFile,
        requested_name: &str,
        span: &SourceSpan,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        function_scopes(&file.tokens)
            .into_iter()
            .find(|scope| {
                span.start.offset >= scope.body_start
                    && span.start.offset < scope.end
                    && scope.shadows(requested_name, &file.tokens, self.token_index_for_span(file, span).unwrap_or(0))
            })
            .map(|scope| {
                let conflict = scope
                    .shadowing_binding(
                        requested_name,
                        &file.tokens,
                        self.token_index_for_span(file, span).unwrap_or(0),
                    )
                    .map(|binding| scope_shadow_declaration(file, binding))
                    .unwrap_or_else(|| span.clone());
                (
                    workspace_location(conflict),
                    RenameAffectedScope::Lexical {
                        file: file.source.path().as_str().to_string(),
                        start_offset: scope.body_start,
                        end_offset: scope.end,
                    },
                )
            })
    }

    fn token_index_for_span(&self, file: &IndexedFile, span: &SourceSpan) -> Option<usize> {
        file.tokens.iter().position(|token| {
            token.range.start == span.start.offset && token.range.end == span.end.offset
        })
    }

    fn function_declared_at(&self, name: &str, selection: &SourceSpan) -> Option<FunctionSymbol> {
        self.functions
            .iter()
            .find(|symbol| {
                (!symbol.invalid_declaration_name || symbol.package.is_some())
                    && declaration_matches(
                        name,
                        selection,
                        &symbol.name,
                        symbol.package.as_deref(),
                        &symbol.declaration.span,
                    )
            })
            .cloned()
    }

    fn type_declared_at(&self, name: &str, selection: &SourceSpan) -> Option<TypeSymbol> {
        self.types
            .iter()
            .find(|symbol| declaration_matches(name, selection, &symbol.name, symbol.package.as_deref(), &symbol.declaration.span))
            .cloned()
    }

    fn constructor_declared_at(
        &self,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<ConstructorSymbol> {
        self.constructors
            .iter()
            .find(|symbol| declaration_matches(name, selection, &symbol.name, symbol.package.as_deref(), &symbol.declaration.span))
            .cloned()
    }

    fn schema_declared_at(&self, name: &str, selection: &SourceSpan) -> Option<NeutralSymbol> {
        self.schemas
            .iter()
            .find(|symbol| {
                declaration_matches(
                    name,
                    selection,
                    &symbol.name,
                    symbol.package.as_deref(),
                    &symbol.declaration.span,
                )
            })
            .cloned()
    }

    fn schema_alias_declared_at(&self, name: &str, selection: &SourceSpan) -> Option<NeutralSymbol> {
        self.schema_aliases
            .iter()
            .find(|symbol| {
                // Package declarations are retained for canonical adapter
                // declarations, but package-source occurrences are not
                // navigable selections or reference-set members.
                symbol.package.is_none()
                    &&
                declaration_matches(
                    name,
                    selection,
                    &symbol.name,
                    symbol.package.as_deref(),
                    &symbol.declaration.span,
                )
            })
            .cloned()
    }

    fn type_alias_declared_at(&self, name: &str, selection: &SourceSpan) -> Option<TypeAliasSymbol> {
        self.type_aliases
            .iter()
            .find(|symbol| {
                symbol.package.is_none()
                    && declaration_matches(
                        name,
                        selection,
                        &symbol.name,
                        symbol.package.as_deref(),
                        &symbol.declaration.span,
                    )
            })
            .cloned()
    }

    fn effect_declared_at(&self, name: &str, selection: &SourceSpan) -> Option<NeutralSymbol> {
        self.effects
            .iter()
            .find(|symbol| {
                declaration_matches(
                    name,
                    selection,
                    &symbol.name,
                    symbol.package.as_deref(),
                    &symbol.declaration.span,
                )
            })
            .cloned()
    }

    fn handler_declared_at(&self, name: &str, selection: &SourceSpan) -> Option<NeutralSymbol> {
        self.handlers
            .iter()
            .find(|symbol| {
                declaration_matches(
                    name,
                    selection,
                    &symbol.name,
                    symbol.package.as_deref(),
                    &symbol.declaration.span,
                )
            })
            .cloned()
    }

    fn operation_declared_at(
        &self,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<EffectOperationSymbol> {
        self.operations
            .iter()
            .find(|symbol| {
                declaration_matches(
                    name,
                    selection,
                    &symbol.name,
                    symbol.package.as_deref(),
                    &symbol.declaration.span,
                )
            })
            .cloned()
    }

    fn schema_for_reference(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<NeutralSymbol> {
        if let Some(qualifier) = qualifier_for_token(tokens, token_index) {
            return match self.schema_alias_qualified_workspace_module(file, &qualifier) {
                QualifiedWorkspaceModule::Workspace(module) => self
                    .schema_operation_lookup_index
                    .workspace_schemas
                    .get(&(module, name.to_string()))?
                    .iter()
                    .find(|symbol| visible_schema_from_workspace_module(file, symbol))
                    .cloned(),
                QualifiedWorkspaceModule::Ambiguous => None,
                QualifiedWorkspaceModule::External => {
                    self.visible_external_schema_for_qualified_reference(file, &qualifier, name)
                }
                QualifiedWorkspaceModule::Unresolved => None,
            };
        }
        self.visible_schema_for_bare_reference(file, name)
    }

    fn schema_alias_for_reference(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<NeutralSymbol> {
        if let Some(qualifier) = qualifier_for_token(tokens, token_index) {
            return self.visible_schema_alias_for_qualified_reference(file, &qualifier, name);
        }
        self.visible_schema_alias_for_bare_reference(file, name)
    }

    fn effect_for_reference(&self, file: &IndexedFile, name: &str) -> Option<NeutralSymbol> {
        self.effects
            .iter()
            .find(|symbol| {
                symbol.name == name && symbol.module == file.module && symbol.package.is_none()
            })
            .cloned()
    }

    fn handler_for_reference(&self, file: &IndexedFile, name: &str) -> Option<NeutralSymbol> {
        self.handlers
            .iter()
            .find(|symbol| {
                symbol.name == name && symbol.module == file.module && symbol.package.is_none()
            })
            .cloned()
    }

    fn operation_for_qualified_perform(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<EffectOperationSymbol> {
        self.operations
            .iter()
            .find(|symbol| {
                symbol.name == name
                    && symbol.effect_name == qualifier
                    && symbol.module == file.module
                    && symbol.package.is_none()
            })
            .cloned()
    }

}

trait NamedTypeSymbol {
    fn name(&self) -> &str;
    fn module(&self) -> &str;
    fn is_workspace_symbol(&self) -> bool;
}

impl NamedTypeSymbol for TypeSymbol {
    fn name(&self) -> &str {
        &self.name
    }

    fn module(&self) -> &str {
        &self.module
    }

    fn is_workspace_symbol(&self) -> bool {
        self.package.is_none()
    }
}

impl NamedTypeSymbol for TypeAliasSymbol {
    fn name(&self) -> &str {
        &self.name
    }

    fn module(&self) -> &str {
        &self.module
    }

    fn is_workspace_symbol(&self) -> bool {
        self.package.is_none()
    }
}

fn symbol_indices_by_name<T: NamedTypeSymbol>(symbols: &[T]) -> BTreeMap<String, Vec<usize>> {
    let mut by_name = BTreeMap::<String, Vec<usize>>::new();
    for (index, symbol) in symbols.iter().enumerate() {
        by_name
            .entry(symbol.name().to_string())
            .or_default()
            .push(index);
    }
    by_name
}

fn package_symbol_indices_by_name<T: NamedTypeSymbol>(
    symbols: &[T],
) -> BTreeMap<String, Vec<usize>> {
    let mut by_name = BTreeMap::<String, Vec<usize>>::new();
    for (index, symbol) in symbols.iter().enumerate() {
        if !symbol.is_workspace_symbol() {
            by_name
                .entry(symbol.name().to_string())
                .or_default()
                .push(index);
        }
    }
    by_name
}

fn workspace_symbol_indices_by_module_and_name<T: NamedTypeSymbol>(
    symbols: &[T],
) -> BTreeMap<(String, String), Vec<usize>> {
    let mut by_module_and_name = BTreeMap::<(String, String), Vec<usize>>::new();
    for (index, symbol) in symbols.iter().enumerate() {
        if symbol.is_workspace_symbol() {
            by_module_and_name
                .entry((symbol.module().to_string(), symbol.name().to_string()))
                .or_default()
                .push(index);
        }
    }
    by_module_and_name
}

fn package_symbol_indices_by_module_and_name<T: NamedTypeSymbol>(
    symbols: &[T],
) -> BTreeMap<(String, String), Vec<usize>> {
    let mut by_module_and_name = BTreeMap::<(String, String), Vec<usize>>::new();
    for (index, symbol) in symbols.iter().enumerate() {
        if !symbol.is_workspace_symbol() {
            by_module_and_name
                .entry((symbol.module().to_string(), symbol.name().to_string()))
                .or_default()
                .push(index);
        }
    }
    by_module_and_name
}

fn visible_schema_from_workspace_module(file: &IndexedFile, symbol: &NeutralSymbol) -> bool {
    symbol.public
        || symbol.module == file.module
        || file
            .companion_target_module
            .as_ref()
            .is_some_and(|target| target == &symbol.module)
}

fn resolve_qualified_alias(aliases: &BTreeMap<String, String>, qualifier: &str) -> Option<String> {
    let mut parts = qualifier.split("::");
    let alias = parts.next()?;
    let module = aliases.get(alias)?;
    let rest = parts.collect::<Vec<_>>();
    if rest.is_empty() {
        Some(module.clone())
    } else {
        Some(format!("{}::{}", module, rest.join("::")))
    }
}

fn visible_imported_type_for_bare_reference(
    file: &IndexedFile,
    symbol: &TypeSymbol,
    name: &str,
) -> bool {
    symbol.name == name
        && symbol.public
        && match &symbol.package {
            Some(package) => {
                file.external_uses
                    .contains(&(symbol.module.clone(), package.clone()))
                    || symbol.standard_prelude
            }
            None => symbol.module != file.module && file.uses.contains(&symbol.module),
        }
}

fn visible_imported_type_alias_for_bare_reference(
    file: &IndexedFile,
    symbol: &TypeAliasSymbol,
    name: &str,
) -> bool {
    symbol.name == name
        && match &symbol.package {
            Some(_) => symbol.standard_prelude,
            None => symbol.module != file.module && file.uses.contains(&symbol.module),
        }
}

fn visible_type_for_qualified_reference(
    file: &IndexedFile,
    symbol: &TypeSymbol,
    qualified_modules: &[String],
    name: &str,
) -> bool {
    symbol.name == name
        && qualified_modules
            .iter()
            .any(|module| module == &symbol.module)
        && match &symbol.package {
            Some(package) => {
                file.external_uses
                    .contains(&(symbol.module.clone(), package.clone()))
                    || symbol.standard_prelude
            }
            None => symbol.module == file.module || file.uses.contains(&symbol.module),
        }
}

fn visible_type_alias_for_qualified_reference(
    file: &IndexedFile,
    symbol: &TypeAliasSymbol,
    qualified_modules: &[String],
    name: &str,
) -> bool {
    symbol.name == name
        && qualified_modules
            .iter()
            .any(|module| module == &symbol.module)
        && match &symbol.package {
            Some(package) => {
                file.external_uses
                    .contains(&(symbol.module.clone(), package.clone()))
                    || symbol.standard_prelude
            }
            None => symbol.module == file.module || file.uses.contains(&symbol.module),
        }
}

fn resolve_external_qualified_alias(
    aliases: &BTreeMap<String, (String, String)>,
    qualifier: &str,
) -> Option<(String, String)> {
    let mut parts = qualifier.split("::");
    let alias = parts.next()?;
    let (module, package) = aliases.get(alias)?;
    let rest = parts.collect::<Vec<_>>();
    if rest.is_empty() {
        Some((module.clone(), package.clone()))
    } else {
        Some((format!("{}::{}", module, rest.join("::")), package.clone()))
    }
}

fn local_binding_declaration(file: &IndexedFile, binding: &LocalBinding) -> SourceSpan {
    file.source.span(TextRange::new(
        binding.declaration_start,
        binding.declaration_end,
    ))
}

fn scoped_binding_declaration(file: &IndexedFile, binding: &ScopedBinding) -> SourceSpan {
    file.source.span(TextRange::new(
        binding.declaration_start,
        binding.declaration_end,
    ))
}

fn scope_shadow_declaration(file: &IndexedFile, binding: ScopeShadow<'_>) -> SourceSpan {
    match binding {
        ScopeShadow::FunctionBinding(binding) => scoped_binding_declaration(file, binding),
        ScopeShadow::LocalBinding(binding) => local_binding_declaration(file, binding),
    }
}

fn same_scope_binding_conflicts(
    binding: &ClauseBinding,
    selected: &LocalSymbol,
    requested_name: &str,
) -> bool {
    binding.name == requested_name
        && binding.start == selected.scope_start
        && binding.end == selected.scope_end
        && !same_span(&binding.declaration, &selected.declaration)
}
