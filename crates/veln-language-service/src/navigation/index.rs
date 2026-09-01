impl SymbolIndex {
    pub(crate) fn new(
        sources: Vec<SourceFile>,
        dependencies: Vec<DirectDependencySnapshot>,
        standard_library: Option<DirectDependencySnapshot>,
    ) -> Self {
        let mut files = Vec::new();
        let mut declarations = FileDeclarations::default();
        for source in sources {
            let (file, file_declarations) = index_workspace_source(source);
            declarations.extend(file_declarations);
            files.push(file);
        }
        for dependency in dependencies.into_iter().chain(standard_library) {
            index_dependency_sources(&mut files, &mut declarations, dependency);
        }
        attach_classified_path_segments(&mut files);
        Self {
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

    fn symbol_for_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
        prepared_scopes: Option<&[FunctionScope]>,
    ) -> Option<SelectedNavigationSymbol> {
        if let Some(symbol) =
            handler_operation_clause_symbol(file, tokens, token_index, name, selection)
        {
            return Some(SelectedNavigationSymbol::bare(Symbol::Local(symbol)));
        }

        if is_handler_operation_clause_operation_name(tokens, token_index) {
            return None;
        }

        if let Some(symbol) = self.function_declared_at(name, selection) {
            return Some(SelectedNavigationSymbol::bare(Symbol::Function(symbol)));
        }

        if let Some(symbol) = self.type_declared_at(name, selection) {
            return Some(SelectedNavigationSymbol::bare(Symbol::Type(symbol)));
        }

        if let Some(symbol) = self.constructor_declared_at(name, selection) {
            return Some(SelectedNavigationSymbol::bare(Symbol::Constructor(symbol)));
        }

        self.recovery_declared_at(file, tokens, token_index, name, selection)
            .map(|symbol| SelectedNavigationSymbol::bare(Symbol::Recovery(symbol)))
            .or_else(|| {
                self.non_declaration_symbol_for_selection(
                    file,
                    tokens,
                    token_index,
                    name,
                    selection,
                    prepared_scopes,
                )
            })
    }

    fn non_declaration_symbol_for_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
        prepared_scopes: Option<&[FunctionScope]>,
    ) -> Option<SelectedNavigationSymbol> {
        if let Some(segment) =
            self.classified_qualified_segment(file, tokens, token_index, name, selection)
        {
            if let Some(selected) = segment.clone().into_selected_symbol() {
                return Some(selected);
            }
            if let Some(symbol) =
                self.unique_recovery_for_role(file, tokens, token_index, name, segment.segment.role)
            {
                return Some(SelectedNavigationSymbol {
                    symbol: Symbol::Recovery(symbol),
                    classified_path_segment: Some(segment.segment),
                });
            }
        }

        if is_qualified_path_token(tokens, token_index)
            && !is_call_target_token(tokens, token_index)
        {
            return None;
        }

        if !is_call_target_token(tokens, token_index) {
            if let Some(symbol) = local_shadow_symbol(file, tokens, token_index, name) {
                return Some(SelectedNavigationSymbol::bare(Symbol::Local(symbol)));
            }
            return self.type_reference_selection(file, tokens, token_index, name, selection)
                .or_else(|| {
                    self.recovery_type_reference_selection(file, tokens, token_index, name, selection)
                })
                .or_else(|| self.bare_nullary_constructor_selection(file, tokens, token_index, name))
                .or_else(|| {
                    self.recovery_value_binding_selection(file, tokens, token_index, name)
                });
        }
        let Some(qualifier) = qualifier_for_token(tokens, token_index) else {
            if let Some(symbol) = self.symbol_for_bare_call(file, tokens, token_index, name) {
                return Some(SelectedNavigationSymbol::bare(symbol));
            }
            return (!self.bare_call_recovery_blocked(
                file,
                tokens,
                token_index,
                name,
                prepared_scopes,
            ))
                .then(|| self.recovery_bare_call_selection(file, tokens, token_index, name))
                .flatten();
        };
        self.symbol_for_qualified_call(file, &qualifier, name)
            .map(SelectedNavigationSymbol::bare)
    }

    fn type_reference_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<SelectedNavigationSymbol> {
        is_type_reference_token(file, name, selection)
            .then(|| self.visible_type_for_reference(file, tokens, token_index, name))
            .flatten()
            .map(Symbol::Type)
            .map(SelectedNavigationSymbol::bare)
    }

    fn recovery_type_reference_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<SelectedNavigationSymbol> {
        is_type_reference_token_named(file, name, selection)
            .then(|| self.unique_recovery_for_role(file, tokens, token_index, name, NameClass::Type))
            .flatten()
            .map(Symbol::Recovery)
            .map(SelectedNavigationSymbol::bare)
    }

    fn recovery_bare_call_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<SelectedNavigationSymbol> {
        self.unique_recovery_for_roles(
            file,
            tokens,
            token_index,
            name,
            &[NameClass::Constructor, NameClass::Function, NameClass::ValueBinding],
        )
        .map(Symbol::Recovery)
        .map(SelectedNavigationSymbol::bare)
    }

    fn bare_nullary_constructor_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<SelectedNavigationSymbol> {
        is_constructor_reference_token(tokens, token_index)
            .then(|| self.constructor_symbol_for_call(file, tokens, token_index, name))
            .flatten()
            .map(Symbol::Constructor)
            .map(SelectedNavigationSymbol::bare)
    }

    fn recovery_value_binding_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<SelectedNavigationSymbol> {
        if is_field_name(tokens, token_index)
            || is_local_binding_name(tokens, token_index)
            || is_parameter_name(tokens, token_index)
        {
            return None;
        }
        self.unique_recovery_for_role(file, tokens, token_index, name, NameClass::ValueBinding)
            .map(Symbol::Recovery)
            .map(SelectedNavigationSymbol::bare)
    }

    fn classified_qualified_segment(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<ClassifiedNavigationSegment> {
        if previous_non_layout_token(tokens, token_index)
            .is_none_or(|token| token.kind != TokenKind::DoubleColon)
            && next_non_layout_token(tokens, token_index)
                .is_none_or(|token| token.kind != TokenKind::DoubleColon)
        {
            return None;
        }

        if let Some(segment) = file
            .classified_path_segments
            .iter()
            .find(|segment| same_span(&segment.span, selection))
        {
            let symbol = match segment.role {
                NameClass::Type => self
                    .visible_type_for_reference(file, tokens, token_index, name)
                    .map(Symbol::Type),
                NameClass::Constructor => {
                    qualifier_for_token(tokens, token_index)
                        .and_then(|qualifier| {
                            self.constructor_for_qualified_call(file, &qualifier, name)
                        })
                        .map(Symbol::Constructor)
                }
                NameClass::Function => {
                    qualifier_for_token(tokens, token_index)
                        .and_then(|qualifier| self.function_for_qualified_call(file, &qualifier, name))
                        .map(Symbol::Function)
                }
                NameClass::ValueBinding => {
                    qualifier_for_token(tokens, token_index)
                        .and_then(|qualifier| {
                            self.function_for_qualified_call(file, &qualifier, name)
                        })
                        .map(Symbol::Function)
                }
                _ => None,
            };
            return Some(ClassifiedNavigationSegment {
                segment: segment.clone(),
                symbol,
            });
        }
        None
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

    fn recovery_declared_at(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<RecoverySymbol> {
        let roles = recovery_roles_for_declaration_token(tokens, token_index)?;
        let selected = dedup_recovery_symbols(
            file.recovery_symbols
                .iter()
                .filter(|symbol| recovery_matches_declaration(symbol, file, name, &roles, selection))
                .cloned()
                .collect(),
        );
        let selected_symbol = selected.first()?;
        let visible = dedup_recovery_symbols(
            file.recovery_symbols
                .iter()
                .filter(|symbol| recovery_visible_to_selected(symbol, file, name, &roles, &selected))
                .cloned()
                .collect(),
        );
        (visible.len() == 1).then(|| selected_symbol.clone())
    }

    fn unique_recovery_for_role(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        role: NameClass,
    ) -> Option<RecoverySymbol> {
        self.unique_recovery_for_roles(file, tokens, token_index, name, &[role])
    }

    fn unique_recovery_for_roles(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        roles: &[NameClass],
    ) -> Option<RecoverySymbol> {
        let offset = tokens[token_index].range.start;
        let mut candidates = file.recovery_symbols.iter().filter(|symbol| {
            symbol.name == name
                && symbol.source_file == file.source.path().as_str()
                && offset >= symbol.scope_start
                && offset < symbol.scope_end
                && symbol.name_class().is_some_and(|class| {
                    roles
                        .iter()
                        .any(|role| recovery_roles_compatible(class, *role))
                })
        });
        let candidate = candidates.next()?;
        candidates.next().is_none().then(|| candidate.clone())
    }

    fn bare_call_recovery_blocked(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        prepared_scopes: Option<&[FunctionScope]>,
    ) -> bool {
        self.bare_call_shadowed_by_distinct_binding(
            file,
            tokens,
            token_index,
            name,
            prepared_scopes,
        )
            || self.has_visible_non_prelude_imported_function(file, name)
            || self.has_visible_non_prelude_imported_constructor(file, name)
    }

    fn bare_call_shadowed_by_distinct_binding(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        prepared_scopes: Option<&[FunctionScope]>,
    ) -> bool {
        let owned_scopes;
        let scopes = match prepared_scopes {
            Some(scopes) => scopes,
            None => {
                owned_scopes = function_scopes(tokens);
                &owned_scopes
            }
        };
        let Some(shadow) =
            local_binding_shadowing_call_target_in_scopes(scopes, tokens, token_index, name)
        else {
            return false;
        };
        !self.shadow_matches_visible_recovery_binding(file, tokens, token_index, name, &shadow)
    }

    fn shadow_matches_visible_recovery_binding(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        shadow: &ScopeShadow<'_>,
    ) -> bool {
        let offset = tokens[token_index].range.start;
        let (declaration_start, declaration_end) = shadow.declaration_range();
        file.recovery_symbols.iter().any(|symbol| {
            symbol.name == name
                && symbol.source_file == file.source.path().as_str()
                && symbol.kind == SymbolKind::ValueBinding
                && symbol.declaration.start.offset == declaration_start
                && symbol.declaration.end.offset == declaration_end
                && offset >= symbol.scope_start
                && offset < symbol.scope_end
        })
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

fn local_shadow_symbol(
    file: &IndexedFile,
    tokens: &[Token],
    token_index: usize,
    name: &str,
) -> Option<LocalSymbol> {
    if is_field_name(tokens, token_index)
        || is_function_declaration_name(tokens, token_index)
        || is_type_declaration_name(tokens, token_index)
        || is_constructor_declaration_name(tokens, token_index)
        || is_local_binding_name(tokens, token_index)
        || is_handler_operation_clause_operation_name(tokens, token_index)
    {
        return None;
    }
    let offset = tokens[token_index].range.start;
    let scopes = function_scopes(tokens);
    let scope = scopes
        .iter()
        .find(|scope| offset >= scope.body_start && offset < scope.end)?;
    let shadow = scope.shadowing_binding(name, tokens, token_index)?;
    let (scope_start, scope_end, declaration_start, declaration_end) = match shadow {
        ScopeShadow::FunctionBinding(binding) => (
            scope.body_start,
            scope.end,
            binding.declaration_start,
            binding.declaration_end,
        ),
        ScopeShadow::LocalBinding(binding) => (
            binding.start,
            binding.end,
            binding.declaration_start,
            binding.declaration_end,
        ),
    };
    Some(LocalSymbol {
        name: name.to_string(),
        declaration: file
            .source
            .span(TextRange::new(declaration_start, declaration_end)),
        scope_file: file.source.path().as_str().to_string(),
        scope_start,
        scope_end,
        declaration_scope_start: scope_start,
        declaration_scope_end: scope_end,
        kind: LocalSymbolKind::ValueBinding,
    })
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
