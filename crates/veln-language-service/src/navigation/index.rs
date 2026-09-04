impl IndexedDependencies {
    pub(crate) fn new(
        dependencies: Vec<DirectDependencySnapshot>,
        standard_library: Option<DirectDependencySnapshot>,
    ) -> Self {
        let mut files = Vec::new();
        let mut declarations = FileDeclarations::default();
        let mut module = empty_surface_module();
        for dependency in dependencies.into_iter().chain(standard_library) {
            index_dependency_sources(&mut files, &mut declarations, &mut module, dependency);
        }
        attach_classified_path_segments(&mut files, &module, &module);
        Self {
            files,
            declarations,
            module,
        }
    }
}

impl SymbolIndex {
    pub(crate) fn new(sources: Vec<SourceFile>, dependencies: &IndexedDependencies) -> Self {
        let mut files = Vec::new();
        let mut declarations = FileDeclarations::default();
        for source in sources {
            let (file, file_declarations) = index_workspace_source(source);
            declarations.extend(file_declarations);
            files.push(file);
        }
        let workspace_module = merged_surface_module(&files);
        let mut module = workspace_module.clone();
        declarations.extend(dependencies.declarations.clone());
        append_surface_module(&mut module, dependencies.module.clone());
        attach_classified_path_segments(&mut files, &workspace_module, &module);
        files.extend(dependencies.files.clone());
        Self {
            schemas: declarations.schemas,
            effects: declarations.effects,
            handlers: declarations.handlers,
            operations: declarations.operations,
            functions: declarations.functions,
            types: declarations.types,
            constructors: declarations.constructors,
            type_aliases: declarations.type_aliases,
            files,
            function_rename_index: OnceLock::new(),
        }
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
        Some(SymbolRequest {
            index: self,
            symbol: selected.symbol,
            selection,
            classified_path_segment: selected.classified_path_segment,
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
            .find(|symbol| declaration_matches(name, selection, &symbol.name, symbol.package.as_deref(), &symbol.declaration.span))
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

    fn schema_for_reference(&self, file: &IndexedFile, name: &str) -> Option<NeutralSymbol> {
        self.schemas
            .iter()
            .find(|symbol| {
                symbol.name == name && symbol.module == file.module && symbol.package.is_none()
            })
            .cloned()
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
            Some(package) => {
                file.external_uses
                    .contains(&(symbol.module.clone(), package.clone()))
                    || symbol.standard_prelude
            }
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
